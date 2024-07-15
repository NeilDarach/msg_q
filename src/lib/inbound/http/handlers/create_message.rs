use std::fmt::Display;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::domain::messages::models::message::{
    CreateMessageError, CreateMessageRequest, Message, QueueNameEmptyError,
};
use crate::inbound::http::errors::{ApiError, ApiSuccess};
use crate::inbound::http::AppState;

use crate::domain::messages::ports::MessageService;

impl From<CreateMessageError> for ApiError {
    fn from(e: CreateMessageError) -> Self {
        match e {
            CreateMessageError::Unknown(cause) => {
                tracing::error!("{:?}\n{}", cause, cause.backtrace());
                Self::InternalServerError("Internal server error".to_string())
            }
            CreateMessageError::BadQueue(s) => Self::UnprocessableEntity(s.clone()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CreateMessageRequestBody {
    cid: Option<String>,
    content: String,
}

impl CreateMessageRequestBody {
    fn try_into_domain(self) -> Result<CreateMessageRequest, ParseCreateMessageHttpRequestError> {
        let content = &self.content.clone();
        let cid = match &self.cid {
            None => None,
            Some(s) => Some(
                uuid::Uuid::try_parse(s)
                    .map_err(|_| ParseCreateMessageHttpRequestError::BadUuid(s.to_string()))?,
            ),
        };
        Ok(CreateMessageRequest::new(content.clone(), cid))
    }
}

#[derive(Debug, Clone, Error)]
enum ParseCreateMessageHttpRequestError {
    #[error(transparent)]
    QueueName(#[from] QueueNameEmptyError),
    BadUuid(String),
}

impl Display for ParseCreateMessageHttpRequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Create message parse error")
    }
}

impl From<ParseCreateMessageHttpRequestError> for ApiError {
    fn from(e: ParseCreateMessageHttpRequestError) -> Self {
        let message = match e {
            ParseCreateMessageHttpRequestError::QueueName(_) => {
                "queue name cannot be empty".to_string()
            }
            ParseCreateMessageHttpRequestError::BadUuid(s) => {
                format!("{} cannot be parsed to a Uuid", s)
            }
        };
        Self::UnprocessableEntity(message)
    }
}

pub async fn create_message<MS: MessageService>(
    State(state): State<AppState<MS>>,
    Path(queue_name): Path<String>,
    Json(body): Json<CreateMessageRequestBody>,
) -> Result<ApiSuccess<CreateMessageResponseData>, ApiError> {
    let domain_req = body.try_into_domain()?;
    let queue_name = queue_name
        .clone()
        .try_into()
        .map_err(|_| CreateMessageError::BadQueue(queue_name.clone()))?;
    state
        .message_service
        .create_message(queue_name, &domain_req)
        .await
        .map_err(ApiError::from)
        .map(|ref message| ApiSuccess::new(StatusCode::CREATED, message.into()))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CreateMessageResponseData {
    id: String,
}

impl From<&Message> for CreateMessageResponseData {
    fn from(message: &Message) -> Self {
        Self {
            id: message.mid().to_string(),
        }
    }
}
