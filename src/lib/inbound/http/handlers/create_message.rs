use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use axum::response::{IntoResponse,Response};
use serde::{Deserialize,Serialize};
use thiserror::Error;

use crate::domain::messages::models::message::{
  Message, QueueName, QueueNameEmptyError, CreateMessageRequest,
  };

use crate::domain::messages::models::message::CreateMessageError;
use crate::domain::messages::ports::MessageService;
use crate::inbound::http::AppState;

#[derive(Debug,Clone)]
pub struct ApiSuccess<T: Serialize + PartialEq>(StatusCode, Json<ApiResponseBody<T>>);

impl<T> PartialEq for ApiSuccess<T>
where
  T: Serialize + PartialEq,
  {
  fn eq(&self, other: &Self) -> bool {
    self.0 == other.0 && self.1.0 == other.1.0
    }
  }

impl<T> ApiSuccess<T>
where
  T: Serialize + PartialEq,
  {
  fn new(status: StatusCode, data: T) -> Self {
    ApiSuccess(status,Json(ApiResponseBody::new(status,data)))
    }
  }

impl<T> IntoResponse for ApiSuccess<T>
where
  T: Serialize + PartialEq,
  {
  fn into_response(self) -> Response {
    (self.0, self.1).into_response()
    }
  }

#[derive(Debug,Clone,PartialEq,Eq)]
pub enum ApiError {
  InternalServerError(String),
  UnprocessableEntity(String),
  }

impl From<anyhow::Error> for ApiError {
  fn from(e: anyhow::Error) -> Self {
    Self::InternalServerError(e.to_string())
    }
  }

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

impl IntoResponse for ApiError {
  fn into_response(self) -> Response {
    use ApiError::*;
    match self {
      InternalServerError(e) => {
        tracing::error!("{}", e);
        (
          StatusCode::INTERNAL_SERVER_ERROR,
          Json(ApiResponseBody::new_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal server error".to_string(),
            )),
         ).into_response()
      }
      UnprocessableEntity(message) => (
          StatusCode::UNPROCESSABLE_ENTITY,
          Json(ApiResponseBody::new_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            message,
            )),
         ).into_response(),
    }
  }
}

#[derive(Debug,Clone,PartialEq,Eq,Serialize)]
pub struct ApiResponseBody<T: Serialize + PartialEq> {
  status_code: u16,
  data: T,
  }

impl<T: Serialize + PartialEq> ApiResponseBody<T> {
  pub fn new(status_code: StatusCode, data: T) -> Self {
    Self { status_code: status_code.as_u16(), data }
  }
}

impl ApiResponseBody<ApiErrorData> {
  pub fn new_error(status_code: StatusCode, message: String) -> Self {
    Self {
      status_code: status_code.as_u16(),
      data: ApiErrorData { message },
      }
  }
}

#[derive(Debug,Clone,PartialEq,Eq,Serialize)]
pub struct ApiErrorData {
  pub message: String,
}

#[derive(Debug,Clone,PartialEq,Eq,Deserialize)]
pub struct CreateMessageRequestBody {
  queue_name: String,
  content: String,
  }

impl CreateMessageRequestBody {
  fn try_into_domain(self) -> Result<CreateMessageRequest,ParseCreateMessageHttpRequestError> {
    let queue_name = QueueName::new(&self.queue_name)?;
    let content = &self.content.clone();
    Ok(CreateMessageRequest::new(queue_name,content.clone()))
  }
}

#[derive(Debug,Clone,Error)]
enum ParseCreateMessageHttpRequestError {
  #[error(transparent)]
  QueueName(#[from] QueueNameEmptyError),
}

impl From<ParseCreateMessageHttpRequestError> for ApiError {
  fn from(e: ParseCreateMessageHttpRequestError) -> Self {
    let message = match e {
      ParseCreateMessageHttpRequestError::QueueName(_) => "queue name cannot be empty".to_string()
      };
    Self::UnprocessableEntity(message)
    }
}

pub async fn create_message<MS: MessageService>(
  State(state):  State<AppState<MS>>,
  Json(body): Json<CreateMessageRequestBody>,
  ) -> Result<ApiSuccess<CreateMessageResponseData>, ApiError> {
    let domain_req = body.try_into_domain()?;
    state
      .message_service
      .create_message(&domain_req)
      .await
      .map_err(ApiError::from)
      .map(|ref message| ApiSuccess::new(StatusCode::CREATED,message.into()))
  }



#[derive(Debug,Clone,PartialEq,Eq,Serialize)]
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



