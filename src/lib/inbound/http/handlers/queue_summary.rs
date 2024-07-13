use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::Serialize;

use crate::domain::messages::models::message::{QueueName, QueueSummary, QueueSummaryError};
use crate::domain::messages::ports::MessageService;
use crate::inbound::http::errors::{ApiError, ApiSuccess};
use crate::inbound::http::AppState;

impl From<QueueSummaryError> for ApiError {
    fn from(e: QueueSummaryError) -> Self {
        match e {
            QueueSummaryError::Unknown(e) => Self::InternalServerError(e.to_string()),
        }
    }
}

pub async fn queue_summary<MS: MessageService>(
    State(state): State<AppState<MS>>,
    queue_name: Option<Path<String>>,
) -> Result<ApiSuccess<Vec<QueueSummaryResponseData>>, ApiError> {
    let queue_name = match queue_name {
        Some(Path(s)) => Some(QueueName::new(&s)?),
        None => None,
    };

    state
        .message_service
        .queue_summary(queue_name)
        .await
        .map_err(ApiError::from)
        .map(|ref summaries| {
            ApiSuccess::new(
                StatusCode::OK,
                summaries.iter().map(|e| e.into()).collect::<Vec<_>>(),
            )
        })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QueueSummaryResponseData {
    queue_name: String,
    depth: usize,
}

impl From<&QueueSummary> for QueueSummaryResponseData {
    fn from(summary: &QueueSummary) -> Self {
        Self {
            queue_name: summary.queue_name().to_string(),
            depth: summary.depth(),
        }
    }
}
