use crate::domain::messages::models::message::{Message,CreateMessageRequest};
use crate::domain::messages::models::message::CreateMessageError;
use crate::domain::messages::ports::{MessageRepository,MessageService};

#[derive(Debug,Clone)]
pub struct Service<R>
where
  R: MessageRepository,
  {
  repo: R,
  }

impl<R> Service<R>
where
  R: MessageRepository,
  {
  pub fn new(repo: R) -> Self {
    Self { repo }
    }
  }

impl<R> MessageService for Service<R>
where
  R: MessageRepository,
  {
  async fn create_message(&self, req: &CreateMessageRequest) -> Result<Message,CreateMessageError> {
    let result = self.repo.create_message(req).await;
    result
    }
  }
      
