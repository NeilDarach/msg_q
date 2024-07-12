use axum::extract::State;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::Json;
use axum::response::{IntoResponse,Response};
use crate::domain::messages::models::message::GetMessageError;
//use serde::Deserialize;
use serde::Serialize;
//use thiserror::Error;

use crate::domain::messages::models::message::{
  Message, QueueName, QueueNameEmptyError, 
  };

//use crate::domain::messages::models::message::GetMessageError;
use crate::domain::messages::ports::MessageService;
use crate::inbound::http::AppState;

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

impl From<GetMessageError> for ApiError {
  fn from(e: GetMessageError) -> Self {
    Self::InternalServerError(e.to_string())
    }
  }
  
impl From<anyhow::Error> for ApiError {
  fn from(e: anyhow::Error) -> Self {
    Self::InternalServerError(e.to_string())
    }
  }

impl From<QueueNameEmptyError> for ApiError {
  fn from(_e: QueueNameEmptyError) -> Self {
    Self::UnprocessableEntity("Queue name cannot be empty".to_string())
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



pub async fn get_message<MS: MessageService>(
  State(state):  State<AppState<MS>>,
  Path((queue_name,id)): Path<(String,String)>,
  ) -> Result<ApiSuccess<GetMessageResponseData>, ApiError> {
    let queue_name = QueueName::new(queue_name.as_str())?;
    state
      .message_service
      .get_message(queue_name,&id)
      .await
      .map_err(ApiError::from)
      .map(|ref message| ApiSuccess::new(StatusCode::CREATED,message.into()))
  }



#[derive(Debug,Clone,PartialEq,Eq,Serialize)]
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



