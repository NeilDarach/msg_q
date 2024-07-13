use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::Serialize;

use crate::domain::messages::models::message::{
    GetMessageError, Message, QueueName, QueueNameEmptyError,
};
use crate::domain::messages::ports::MessageService;
use crate::inbound::http::errors::{ApiError, ApiSuccess};
use crate::inbound::http::AppState;

impl From<GetMessageError> for ApiError {
    fn from(e: GetMessageError) -> Self {
        match e {
            GetMessageError::NoMessage(e) => Self::NotFound(e),
            GetMessageError::BadUuid(e) => Self::UnprocessableEntity(format!("Bad uuid {}", e)),
            GetMessageError::Unknown(e) => Self::InternalServerError(e.to_string()),
        }
    }
}

impl From<QueueNameEmptyError> for ApiError {
    fn from(_e: QueueNameEmptyError) -> Self {
        Self::UnprocessableEntity("Queue name cannot be empty".to_string())
    }
}

pub async fn get_next_message<MS: MessageService>(
    State(state): State<AppState<MS>>,
    Path(queue_name): Path<String>,
) -> Result<ApiSuccess<GetMessageResponseData>, ApiError> {
    let queue_name = QueueName::new(queue_name.as_str())?;
    state
        .message_service
        .get_next_message(queue_name)
        .await
        .map_err(ApiError::from)
        .map(|ref message| ApiSuccess::new(StatusCode::OK, message.into()))
}

pub async fn get_message<MS: MessageService>(
    State(state): State<AppState<MS>>,
    Path((queue_name, id)): Path<(String, String)>,
) -> Result<ApiSuccess<GetMessageResponseData>, ApiError> {
    let queue_name = QueueName::new(queue_name.as_str())?;
    state
        .message_service
        .get_message(queue_name, &id)
        .await
        .map_err(ApiError::from)
        .map(|ref message| ApiSuccess::new(StatusCode::OK, message.into()))
}

pub async fn browse_next_message<MS: MessageService>(
    State(state): State<AppState<MS>>,
    Path(queue_name): Path<String>,
) -> Result<ApiSuccess<GetMessageResponseData>, ApiError> {
    let queue_name = QueueName::new(queue_name.as_str())?;
    state
        .message_service
        .browse_next_message(queue_name)
        .await
        .map_err(ApiError::from)
        .map(|ref message| ApiSuccess::new(StatusCode::OK, message.into()))
}

pub async fn browse_message<MS: MessageService>(
    State(state): State<AppState<MS>>,
    Path((queue_name, id)): Path<(String, String)>,
) -> Result<ApiSuccess<GetMessageResponseData>, ApiError> {
    let queue_name = QueueName::new(queue_name.as_str())?;
    state
        .message_service
        .browse_message(queue_name, &id)
        .await
        .map_err(ApiError::from)
        .map(|ref message| ApiSuccess::new(StatusCode::OK, message.into()))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GetMessageResponseData {
    id: String,
    content: String,
}

impl From<&Message> for GetMessageResponseData {
    fn from(message: &Message) -> Self {
        Self {
            id: message.id().to_string(),
            content: message.content().clone(),
        }
    }
}
