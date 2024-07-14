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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::messages::models::message::{
        CreateMessageError, QueueName, QueueSummary, QueueSummaryError,
    };
    use anyhow::anyhow;
    use std::mem;
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    #[derive(Clone)]
    struct MockMessageService {
        create_message_result: Arc<Mutex<Option<Result<Message, CreateMessageError>>>>,
        get_message_result: Arc<Mutex<Option<Result<Message, GetMessageError>>>>,
        queue_summary_result: Arc<Mutex<Option<Result<Vec<QueueSummary>, QueueSummaryError>>>>,
    }

    impl MockMessageService {
        pub fn new_create(res: Result<Message, CreateMessageError>) -> Self {
            Self {
                create_message_result: Arc::new(Mutex::new(Some(res))),
                get_message_result: Arc::new(Mutex::new(None)),
                queue_summary_result: Arc::new(Mutex::new(None)),
            }
        }
        pub fn new_get(res: Result<Message, GetMessageError>) -> Self {
            Self {
                create_message_result: Arc::new(Mutex::new(None)),
                get_message_result: Arc::new(Mutex::new(Some(res))),
                queue_summary_result: Arc::new(Mutex::new(None)),
            }
        }
        pub fn new_summary(res: Result<Vec<QueueSummary>, QueueSummaryError>) -> Self {
            Self {
                create_message_result: Arc::new(Mutex::new(None)),
                get_message_result: Arc::new(Mutex::new(None)),
                queue_summary_result: Arc::new(Mutex::new(Some(res))),
            }
        }

        pub fn create(&self) -> Result<Message, CreateMessageError> {
            let mut guard = self.create_message_result.lock();
            let mut result = Err(CreateMessageError::Unknown(anyhow!("substitute error")));
            let t = guard.as_deref_mut().unwrap().as_mut().unwrap();
            mem::swap(t, &mut result);

            result
        }
        pub fn get(&self) -> Result<Message, GetMessageError> {
            let mut guard = self.get_message_result.lock();
            let mut result = Err(GetMessageError::Unknown(anyhow!("substitute error")));
            let t = guard.as_deref_mut().unwrap().as_mut().unwrap();
            mem::swap(t, &mut result);

            result
        }
        pub fn summary(&self) -> Result<Vec<QueueSummary>, QueueSummaryError> {
            let mut guard = self.queue_summary_result.lock();
            let mut result = Err(QueueSummaryError::Unknown(anyhow!("substitute error")));
            let t = guard.as_deref_mut().unwrap().as_mut().unwrap();
            mem::swap(t, &mut result);

            result
        }
    }

    impl MessageService for MockMessageService {
        async fn create_message(
            &self,
            _req: &crate::domain::messages::models::message::CreateMessageRequest,
        ) -> Result<Message, CreateMessageError> {
            self.create()
        }

        async fn get_message(
            &self,
            _param: crate::domain::messages::models::message::Parameters,
        ) -> Result<Message, GetMessageError> {
            self.get()
        }

        async fn get_next_message(
            &self,
            _param: crate::domain::messages::models::message::Parameters,
        ) -> Result<Message, GetMessageError> {
            self.get()
        }

        async fn browse_message(
            &self,
            _param: crate::domain::messages::models::message::Parameters,
        ) -> Result<Message, GetMessageError> {
            self.get()
        }

        async fn browse_next_message(
            &self,
            _param: crate::domain::messages::models::message::Parameters,
        ) -> Result<Message, GetMessageError> {
            self.get()
        }

        async fn reserve_message(
            &self,
            _param: crate::domain::messages::models::message::Parameters,
        ) -> Result<Message, GetMessageError> {
            self.get()
        }

        async fn reserve_next_message(
            &self,
            _param: crate::domain::messages::models::message::Parameters,
        ) -> Result<Message, GetMessageError> {
            self.get()
        }

        async fn confirm_message(
            &self,
            _param: crate::domain::messages::models::message::Parameters,
        ) -> Result<Message, GetMessageError> {
            self.get()
        }

        async fn queue_summary(
            &self,
            _queue_name: Option<QueueName>,
        ) -> Result<Vec<QueueSummary>, QueueSummaryError> {
            self.summary()
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_message_success() {
        let queue_name = QueueName::new("test").unwrap();
        let content = "A String".to_string();
        let message_id = Uuid::new_v4();
        let service =
            MockMessageService::new_get(Ok(Message::new(message_id.clone(), content.clone())));
        let state = axum::extract::State(AppState {
            message_service: Arc::new(service),
        });
        let expected = ApiSuccess::new(
            StatusCode::OK,
            GetMessageResponseData {
                id: message_id.to_string(),
                content: content.clone(),
            },
        );

        let path = axum::extract::Path(("test".to_string(), message_id.to_string()));

        let actual = get_message(state, path).await;
        assert!(
            actual.is_ok(),
            "expected create_message to succeed, but got {:?}",
            actual
        );

        let actual = actual.unwrap();
        assert_eq!(
            actual, expected,
            "expected ApiSuccess {:?}, but got {:?}",
            expected, actual
        )
    }
}
