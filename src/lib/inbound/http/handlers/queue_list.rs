use axum::extract::State;
use axum::http::StatusCode;

use crate::domain::messages::models::message::{QueueList, QueueListError};
use crate::domain::messages::ports::MessageService;
use crate::inbound::http::errors::{ApiError, ApiSuccess};
use crate::inbound::http::AppState;

impl From<QueueListError> for ApiError {
    fn from(e: QueueListError) -> Self {
        match e {
            QueueListError::Unknown(e) => Self::InternalServerError(e.to_string()),
        }
    }
}

pub async fn queue_list<MS: MessageService>(
    State(state): State<AppState<MS>>,
) -> Result<ApiSuccess<QueueList>, ApiError> {
    state
        .message_service
        .queue_list()
        .await
        .map_err(ApiError::from)
        .map(|ref mut list| {
            list.0.sort();
            ApiSuccess::new(StatusCode::OK, list.clone())
        })
}
