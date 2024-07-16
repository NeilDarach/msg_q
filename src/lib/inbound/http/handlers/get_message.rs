use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use std::collections::HashMap;

use crate::domain::messages::models::message::{
    GetMessageAction, GetMessageError, GetMessageOptions, Message, QueueSummary, QueueSummaryError,
};
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

impl From<QueueSummaryError> for ApiError {
    fn from(e: QueueSummaryError) -> Self {
        match e {
            QueueSummaryError::Unknown(e) => Self::InternalServerError(e.to_string()),
            QueueSummaryError::NoQueue(e) => Self::NotFound(e.to_string()),
        }
    }
}

pub async fn get_message_mid<MS: MessageService>(
    State(state): State<AppState<MS>>,
    Path((queue_name, mid)): Path<(String, String)>,
    Query(mut params): Query<HashMap<String, String>>,
) -> Result<ApiSuccess<GetMessageResponseData>, ApiError> {
    if params.get("mid").is_some() {
        return Err(GetMessageError::InvalidParameter("mid specified twice".to_string()).into());
    }
    params.insert("queue_name".to_string(), queue_name);
    params.insert("mid".to_string(), mid);
    let params: GetMessageOptions = params.try_into()?;
    if params.action() == GetMessageAction::Query {
        return Err(
            GetMessageError::InvalidParameter("query not valid for a message".to_string()).into(),
        );
    }
    state
        .message_service
        .get_message(params)
        .await
        .map_err(ApiError::from)
        .map(|ref message| ApiSuccess::new(StatusCode::OK, message.into()))
}

pub async fn get_message<MS: MessageService>(
    State(state): State<AppState<MS>>,
    Path(queue_name): Path<String>,
    Query(mut params): Query<HashMap<String, String>>,
) -> Result<Response, ApiError> {
    params.insert("queue_name".to_string(), queue_name);
    let params: GetMessageOptions = params.try_into()?;
    if params.action() == GetMessageAction::Query {
        let ret = state
            .message_service
            .get_info(params)
            .await
            .map_err(ApiError::from)
            .map(|ref message| {
                let message: QueueSummaryResponseData = message.into();
                ApiSuccess::new(StatusCode::OK, message)
            });
        return ret.map(|r| r.into_response());
    }
    state
        .message_service
        .get_message(params)
        .await
        .map_err(ApiError::from)
        .map(|ref message| {
            let message: GetMessageResponseData = message.into();
            ApiSuccess::new(StatusCode::OK, message)
        })
        .map(|r| r.into_response())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GetMessageResponseData {
    mid: String,
    cid: Option<String>,
    cursor: usize,
    content: String,
}

impl From<&Message> for GetMessageResponseData {
    fn from(message: &Message) -> Self {
        Self {
            mid: message.mid().to_string(),
            cid: message.cid().map(|uid| uid.to_string()),
            cursor: message.cursor(),
            content: message.content().clone(),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::messages::models::message::{
        CreateMessageError, CreateMessageRequest, QueueList, QueueListError, QueueName,
    };
    use anyhow::anyhow;
    use std::mem;
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    #[derive(Clone)]
    struct MockMessageService {
        get_message_result: Arc<Mutex<Option<Result<Message, GetMessageError>>>>,
    }

    impl MockMessageService {
        pub fn new_get(res: Result<Message, GetMessageError>) -> Self {
            Self {
                get_message_result: Arc::new(Mutex::new(Some(res))),
            }
        }

        pub fn get(&self) -> Result<Message, GetMessageError> {
            let mut guard = self.get_message_result.lock();
            let mut result = Err(GetMessageError::Unknown(anyhow!("substitute error")));
            let t = guard.as_deref_mut().unwrap().as_mut().unwrap();
            mem::swap(t, &mut result);

            result
        }
    }

    impl MessageService for MockMessageService {
        async fn create_message(
            &self,
            _queue_name: QueueName,
            _req: &CreateMessageRequest,
        ) -> Result<Message, CreateMessageError> {
            todo!()
        }

        async fn get_message(&self, _param: GetMessageOptions) -> Result<Message, GetMessageError> {
            self.get()
        }

        async fn queue_list(&self) -> Result<QueueList, QueueListError> {
            todo!()
        }
        async fn get_info(
            &self,
            _param: GetMessageOptions,
        ) -> Result<QueueSummary, QueueSummaryError> {
            todo!()
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_message_success() {
        let content = "A String".to_string();
        let message_id = Uuid::new_v4();
        let service =
            MockMessageService::new_get(Ok(Message::new(message_id, None, content.clone())));
        let state = axum::extract::State(AppState {
            message_service: Arc::new(service),
        });
        let _expected = ApiSuccess::new(
            StatusCode::OK,
            GetMessageResponseData {
                mid: message_id.to_string(),
                cid: None,
                cursor: 0,
                content: content.clone(),
            },
        );

        let path = axum::extract::Path("test".to_string());
        let mut gmo = HashMap::new();
        gmo.insert("action".to_string(), "browse".to_string());

        let actual = get_message(state, path, axum::extract::Query(gmo)).await;
        assert!(
            actual.is_ok(),
            "expected create_message to succeed, but got {:?}",
            actual
        );

        let _actual = actual.unwrap();
        //assert_eq!(
        //actual, expected,
        //"expected ApiSuccess {:?}, but got {:?}",
        //expected, actual
        //)
    }
}
