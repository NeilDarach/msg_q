//use std::str::FromStr;
use std::sync::Arc;
use std::sync::Mutex;
use std::collections::HashMap;

//use anyhow::{anyhow, Context};
use uuid::Uuid;

use crate::domain::messages::models::message::{Message,QueueName,CreateMessageRequest};
use crate::domain::messages::models::message::{CreateMessageError,GetMessageError};
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

fn index_of(queue: &Vec<Message>, id: &String) -> Option<usize> {
      let mut i = 0;
      while i < queue.len() {
        if &queue[i].id().to_string() == id {
          return Some(i);
        } else {
          i += 1;
        }
    }
      None
}
  
impl MessageRepository for Memory {
  async fn get_message(&self, queue_name: QueueName, id: &String) -> Result<Message, GetMessageError> {
    let mut queues = self.queues.lock().unwrap();
    if let Some(queue) = queues.get_mut(&queue_name) {
      if let Some(i) = index_of(queue,id) {
        let msg = queue.remove(i);
        return Ok(msg)
      }
    }
    Err(GetMessageError::NoMessage)
}
    
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
