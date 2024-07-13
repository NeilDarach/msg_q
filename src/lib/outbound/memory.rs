use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::sync::Mutex;
use uuid::Uuid;

use crate::domain::messages::models::message::{
    CreateMessageError, GetMessageError, QueueSummaryError,
};
use crate::domain::messages::models::message::{
    CreateMessageRequest, Message, QueueName, QueueSummary,
};
use crate::domain::messages::ports::MessageRepository;

#[derive(Debug, Clone)]
pub struct Memory {
    queues: Arc<Mutex<HashMap<QueueName, VecDeque<Message>>>>,
}

impl Memory {
    pub async fn new() -> Result<Memory, anyhow::Error> {
        let queues = Arc::new(Mutex::new(HashMap::new()));
        Ok(Self { queues })
    }
}

impl MessageRepository for Memory {
    async fn get_message(
        &self,
        queue_name: QueueName,
        id: &str,
    ) -> Result<Message, GetMessageError> {
        if let Ok(id) = Uuid::parse_str(id) {
            let mut queues = self.queues.lock().unwrap();
            if let Some(queue) = queues.get_mut(&queue_name) {
                if let Some(i) = queue.iter().position(|e| e.id() == &id) {
                    let msg = queue.remove(i).unwrap();
                    return Ok(msg);
                }
            }
        } else {
            return Err(GetMessageError::BadUuid(id.to_string()));
        }
        Err(GetMessageError::NoMessage(format!("{}/{}", queue_name, id)))
    }

    async fn get_next_message(&self, queue_name: QueueName) -> Result<Message, GetMessageError> {
        let mut queues = self.queues.lock().unwrap();
        if let Some(queue) = queues.get_mut(&queue_name) {
            if let Some(msg) = queue.pop_front() {
                return Ok(msg);
            }
        }
        Err(GetMessageError::NoMessage(format!("{}", queue_name)))
    }

    async fn browse_message(
        &self,
        queue_name: QueueName,
        id: &str,
    ) -> Result<Message, GetMessageError> {
        if let Ok(id) = Uuid::parse_str(id) {
            let mut queues = self.queues.lock().unwrap();
            if let Some(queue) = queues.get_mut(&queue_name) {
                if let Some(i) = queue.iter().position(|e| e.id() == &id) {
                    let msg = queue.get(i).unwrap();
                    return Ok(msg.clone());
                }
            }
        } else {
            return Err(GetMessageError::BadUuid(id.to_string()));
        }
        Err(GetMessageError::NoMessage(format!("{}/{}", queue_name, id)))
    }

    async fn browse_next_message(&self, queue_name: QueueName) -> Result<Message, GetMessageError> {
        let mut queues = self.queues.lock().unwrap();
        if let Some(queue) = queues.get_mut(&queue_name) {
            if let Some(msg) = queue.get(0) {
                return Ok(msg.clone());
            }
        }
        Err(GetMessageError::NoMessage(format!("{}", queue_name)))
    }

    async fn create_message(
        &self,
        req: &CreateMessageRequest,
    ) -> Result<Message, CreateMessageError> {
        let id = Uuid::new_v4();
        let mut queues = self.queues.lock().unwrap();
        let content = req.content().clone();
        let message = Message::new(id, content);
        let entry = queues.entry(req.queue_name().clone()).or_default();
        entry.push_back(message.clone());
        Ok(message)
    }

    async fn queue_summary(
        &self,
        queue_name: Option<QueueName>,
    ) -> Result<Vec<QueueSummary>, QueueSummaryError> {
        let queues = self.queues.lock().unwrap();

        Ok(queues
            .iter()
            .filter(|(k, _)| queue_name.is_none() || Some(*k) == queue_name.as_ref())
            .map(|(k, v)| QueueSummary::new(k, v.len()))
            .collect::<Vec<_>>())
    }
}
