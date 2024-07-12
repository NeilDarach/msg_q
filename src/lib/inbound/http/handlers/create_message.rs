use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::domain::messages::models::message::{
    CreateMessageError, CreateMessageRequest, Message, QueueName, QueueNameEmptyError,
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
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CreateMessageRequestBody {
    queue_name: String,
    content: String,
}

impl CreateMessageRequestBody {
    fn try_into_domain(self) -> Result<CreateMessageRequest, ParseCreateMessageHttpRequestError> {
        let queue_name = QueueName::new(&self.queue_name)?;
        let content = &self.content.clone();
        Ok(CreateMessageRequest::new(queue_name, content.clone()))
    }
}

#[derive(Debug, Clone, Error)]
enum ParseCreateMessageHttpRequestError {
    #[error(transparent)]
    QueueName(#[from] QueueNameEmptyError),
}

impl From<ParseCreateMessageHttpRequestError> for ApiError {
    fn from(e: ParseCreateMessageHttpRequestError) -> Self {
        let message = match e {
            ParseCreateMessageHttpRequestError::QueueName(_) => {
                "queue name cannot be empty".to_string()
            }
        };
        Self::UnprocessableEntity(message)
    }
}

pub async fn create_message<MS: MessageService>(
    State(state): State<AppState<MS>>,
    Json(body): Json<CreateMessageRequestBody>,
) -> Result<ApiSuccess<CreateMessageResponseData>, ApiError> {
    let domain_req = body.try_into_domain()?;
    state
        .message_service
        .create_message(&domain_req)
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
            id: message.id().to_string(),
        }
    }
}
