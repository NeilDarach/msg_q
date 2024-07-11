use std::str::FromStr;
use std::sync::Arc;
use std::sync::Mutex;
use std::collections::HashMap;

use anyhow::{anyhow, Context};
use uuid::Uuid;

use crate::domain::messages::models::message::{Message,QueueName,CreateMessageRequest};
use crate::domain::messages::models::message::CreateMessageError;
use crate::domain::messages::ports::MessageRepository;

#[derive(Debug,Clone)]
pub struct Memory {
  queues: Arc<Mutex<HashMap<QueueName,Vec<Message>>>>,
}

impl Memory {
  pub async fn new() -> Result<Memory, anyhow::Error> {
    let queues = Arc::new(Mutex::new(HashMap::new()));
    Ok(Self { queues })
    }
  }

impl MessageRepository for Memory {
  async fn create_message(&self, req: &CreateMessageRequest) -> Result<Message, CreateMessageError> {
    let id = Uuid::new_v4();
    let mut queues = self.queues.lock().unwrap();
    let content = req.content().clone();
    let message = Message::new(id, content);
    let entry = queues.entry(req.queue_name().clone()).or_insert(Vec::new()); 
    entry.push(message.clone());
    Ok(message)
    }
  } 
