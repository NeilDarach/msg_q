use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
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
) -> Result<ApiSuccess<GetMessageReturnType>, ApiError> {
    params.insert("queue_name".to_string(), queue_name);
    let params: GetMessageOptions = params.try_into()?;
    if params.action() == GetMessageAction::Query {
        return state
            .message_service
            .get_info(params)
            .await
            .map_err(ApiError::from)
            .map(|ref info| {
                ApiSuccess::new(StatusCode::OK, GetMessageReturnType::Info(info.into()))
            });
    }
    state
        .message_service
        .get_message(params)
        .await
        .map_err(ApiError::from)
        .map(|ref message| {
            ApiSuccess::new(
                StatusCode::OK,
                GetMessageReturnType::Message(message.into()),
            )
        })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum GetMessageReturnType {
    Message(GetMessageResponseData),
    Info(QueueSummaryResponseData),
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
    use serde_json;
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
            let mut result = Err(GetMessageError::Unknown(Arc::new(anyhow!(
                "substitute error"
            ))));
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
            unreachable!()
        }

        async fn get_message(&self, _param: GetMessageOptions) -> Result<Message, GetMessageError> {
            self.get()
        }

        async fn queue_list(&self) -> Result<QueueList, QueueListError> {
            unreachable!()
        }
        async fn get_info(
            &self,
            _param: GetMessageOptions,
        ) -> Result<QueueSummary, QueueSummaryError> {
            unreachable!()
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_message_success() {
        let content = "A String".to_string();
        let message_id = Uuid::new_v4();
        let response = Ok(Message::new(message_id, None, content.clone(), None));
        let expected = ApiSuccess::new(
            StatusCode::OK,
            GetMessageReturnType::Message(GetMessageResponseData {
                mid: message_id.to_string(),
                cid: None,
                cursor: 0,
                content: content.clone(),
            }),
        );
        let actual = get("test", r#"{"action":"browse"}"#, &response)
            .await
            .unwrap();

        assert_eq!(actual, expected)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_message_bad_mid() {
        let response = Ok(Message::new(Uuid::new_v4(), None, "".to_string(), None));
        let expected = ApiError::UnprocessableEntity("Bad parameter mid".to_string());
        let actual = get("test", r#"{"action":"browse","mid":"xxx"}"#, &response).await;
        assert_eq!(actual, Err(expected));

        let actual = get(
            "test",
            r#"{"action":"browse","mid":"61fb8b36-c7e6-4a34-af8a-011a73f065f0"}"#,
            &response,
        )
        .await;
        assert!(actual.is_ok(), "{:?}", actual);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_message_bad_reservation() {
        let response = Ok(Message::new(Uuid::new_v4(), None, "".to_string(), None));
        let expected =
            ApiError::UnprocessableEntity("Bad parameter reservation_seconds".to_string());
        let actual = get(
            "test",
            r#"{"action":"get","reservation_seconds":"xxx"}"#,
            &response,
        )
        .await;
        assert_eq!(actual, Err(expected));

        let expected =
            ApiError::UnprocessableEntity("Bad parameter reservation_seconds".to_string());
        let actual = get(
            "test",
            r#"{"action":"browse","reservation_seconds":"10"}"#,
            &response,
        )
        .await;
        assert_eq!(actual, Err(expected));

        let actual = get(
            "test",
            r#"{"action":"get","reservation_seconds":"10"}"#,
            &response,
        )
        .await;
        assert!(actual.is_ok(), "{:?}", actual);
    }

    async fn get(
        path: &str,
        gmo: &str,
        response: &Result<Message, GetMessageError>,
    ) -> Result<ApiSuccess<GetMessageReturnType>, ApiError> {
        let service = MockMessageService::new_get(response.clone());
        let state = axum::extract::State(AppState {
            message_service: Arc::new(service),
        });

        let path = axum::extract::Path(path.to_string());
        let gmo = serde_json::from_str(gmo).unwrap();

        get_message(state, path, axum::extract::Query(gmo)).await
    }
}
