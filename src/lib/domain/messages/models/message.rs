use std::fmt::{Display,Formatter};

use derive_more::From;
use thiserror::Error;

#[derive(Clone,Debug,PartialEq,Eq,PartialOrd,Ord,Hash)]
pub struct Message {
  id: uuid::Uuid,
  content: String
  }


impl Message { 
  pub fn new(id: uuid::Uuid, content: String) -> Self {
    Self { id, content }
  }

  pub fn id(&self) -> &uuid::Uuid {
    &self.id
  }

  pub fn content(&self) -> &String {
    &self.content
  }
}

#[derive(Clone,Debug,PartialEq,Eq,PartialOrd,Ord,Hash)]
pub struct QueueName(String);

#[derive(Clone,Debug,Error)]
#[error("queue name cannot be empty")]
pub struct QueueNameEmptyError;

impl QueueName {
  pub fn new(raw: &str) -> Result<Self,QueueNameEmptyError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
      Err(QueueNameEmptyError)
    } else {
      Ok(Self(trimmed.to_string()))
    }
  }
}

impl Display for QueueName {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.write_str(&self.0)
  }
}

#[derive(Clone,Debug,PartialEq,Eq,PartialOrd,Ord,Hash,From)]
pub struct CreateMessageRequest {
  queue_name: QueueName,
  content: String,
}

impl CreateMessageRequest {
  pub fn new(queue_name: QueueName, content: String) -> Self {
    Self { queue_name, content }
  }

 pub fn queue_name(&self) -> &QueueName {
  &self.queue_name
  }

  pub fn content(&self) -> &String {
    &self.content
  }
}

#[derive(Debug,Error)]
pub enum CreateMessageError {
  #[error(transparent)]
  Unknown(#[from] anyhow::Error),
  }


