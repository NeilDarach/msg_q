use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::Serialize;
use std::collections::HashMap;

use crate::domain::messages::models::message::{GetMessageError, Message, QueueNameEmptyError};
use crate::domain::messages::ports::MessageService;
use crate::inbound::http::errors::{ApiError, ApiSuccess};
use crate::inbound::http::AppState;

impl From<GetMessageError> for ApiError {
    fn from(e: GetMessageError) -> Self {
        match e {
            GetMessageError::NoMessage(e) => Self::NotFound(e),
            GetMessageError::BadUuid(e) => Self::UnprocessableEntity(format!("Bad uuid {}", e)),
            GetMessageError::MissingParameter(e) => {
                Self::UnprocessableEntity(format!("Missing parameter {}", e))
            }
            GetMessageError::InvalidParameter(e) => {
                Self::UnprocessableEntity(format!("Bad parameter {}", e))
            }
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
    let mut params = HashMap::new();
    params.insert("queue_name".to_string(), queue_name);
    params.insert("remove".to_string(), "true".to_string());

    state
        .message_service
        .get_next_message(params.try_into()?)
        .await
        .map_err(ApiError::from)
        .map(|ref message| ApiSuccess::new(StatusCode::OK, message.into()))
}

pub async fn get_message<MS: MessageService>(
    State(state): State<AppState<MS>>,
    Path((queue_name, id)): Path<(String, String)>,
) -> Result<ApiSuccess<GetMessageResponseData>, ApiError> {
    let mut params = HashMap::new();
    params.insert("queue_name".to_string(), queue_name);
    params.insert("remove".to_string(), "true".to_string());
    params.insert("id".to_string(), id);
    state
        .message_service
        .get_message(params.try_into()?)
        .await
        .map_err(ApiError::from)
        .map(|ref message| ApiSuccess::new(StatusCode::OK, message.into()))
}

pub async fn browse_next_message<MS: MessageService>(
    State(state): State<AppState<MS>>,
    Path(queue_name): Path<String>,
) -> Result<ApiSuccess<GetMessageResponseData>, ApiError> {
    let mut params = HashMap::new();
    params.insert("queue_name".to_string(), queue_name);
    params.insert("remove".to_string(), "false".to_string());
    state
        .message_service
        .browse_next_message(params.try_into()?)
        .await
        .map_err(ApiError::from)
        .map(|ref message| ApiSuccess::new(StatusCode::OK, message.into()))
}

pub async fn browse_message<MS: MessageService>(
    State(state): State<AppState<MS>>,
    Path((queue_name, id)): Path<(String, String)>,
) -> Result<ApiSuccess<GetMessageResponseData>, ApiError> {
    let mut params = HashMap::new();
    params.insert("queue_name".to_string(), queue_name);
    params.insert("remove".to_string(), "false".to_string());
    params.insert("id".to_string(), id);

    tracing::info!("map: {:?}", params);
    state
        .message_service
        .browse_message(params.try_into()?)
        .await
        .map_err(ApiError::from)
        .map(|ref message| ApiSuccess::new(StatusCode::OK, message.into()))
}

pub async fn reserve_next_message<MS: MessageService>(
    State(state): State<AppState<MS>>,
    Path(queue_name): Path<String>,
    Query(mut params): Query<HashMap<String, String>>,
) -> Result<ApiSuccess<GetMessageResponseData>, ApiError> {
    //let queue_name = QueueName::new(queue_name.as_str())?;
    params.insert("queue_name".to_string(), queue_name);
    state
        .message_service
        .reserve_next_message(params.try_into()?)
        .await
        .map_err(ApiError::from)
        .map(|ref message| ApiSuccess::new(StatusCode::OK, message.into()))
}
pub async fn reserve_message<MS: MessageService>(
    State(state): State<AppState<MS>>,
    Path((queue_name, id)): Path<(String, String)>,
    Query(mut params): Query<HashMap<String, String>>,
) -> Result<ApiSuccess<GetMessageResponseData>, ApiError> {
    //let queue_name = QueueName::new(queue_name.as_str())?;
    params.insert("queue_name".to_string(), queue_name);
    params.insert("id".to_string(), id);
    tracing::info!("map: {:?}", params);
    state
        .message_service
        .reserve_message(params.try_into()?)
        .await
        .map_err(ApiError::from)
        .map(|ref message| ApiSuccess::new(StatusCode::OK, message.into()))
}

pub async fn confirm_message<MS: MessageService>(
    State(state): State<AppState<MS>>,
    Path((queue_name, id)): Path<(String, String)>,
    Query(mut params): Query<HashMap<String, String>>,
) -> Result<ApiSuccess<GetMessageResponseData>, ApiError> {
    //let queue_name = QueueName::new(queue_name.as_str())?;
    params.insert("queue_name".to_string(), queue_name);
    params.insert("id".to_string(), id);
    params.insert("remove".to_string(), "true".to_string());
    state
        .message_service
        .confirm_message(params.try_into()?)
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
