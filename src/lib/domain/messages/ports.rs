use std::future::Future;

use crate::domain::messages::models::message::{Message, CreateMessageRequest};

#[allow(unused_imports)]
use crate::domain::messages::models::message::QueueName;
use crate::domain::messages::models::message::{GetMessageError,CreateMessageError};

pub trait MessageService: Clone + Send + Sync + 'static {
  fn create_message(&self, req: &CreateMessageRequest) -> impl Future<Output = Result<Message, CreateMessageError>> + Send;
  fn get_message(&self, queue_name: QueueName, id: &String) -> impl Future<Output = Result<Message, GetMessageError>> + Send;
  }

pub trait MessageRepository: Send + Sync + Clone + 'static {
  fn create_message(&self, req: &CreateMessageRequest) -> impl Future<Output = Result<Message, CreateMessageError>> + Send;
  fn get_message(&self, queue_name: QueueName, id: &String) -> impl Future<Output = Result<Message, GetMessageError>> + Send;
}
