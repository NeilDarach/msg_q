use std::collections::HashMap;
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
    queues: Arc<Mutex<HashMap<QueueName, Vec<Message>>>>,
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
        id: &String,
    ) -> Result<Message, GetMessageError> {
        if let Ok(id) = Uuid::parse_str(id) {
            let mut queues = self.queues.lock().unwrap();
            if let Some(queue) = queues.get_mut(&queue_name) {
                if let Some(i) = queue.iter().position(|e| e.id() == &id) {
                    let msg = queue.remove(i);
                    return Ok(msg);
                }
            }
        } else {
            return Err(GetMessageError::BadUuid(id.to_string()));
        }
        Err(GetMessageError::NoMessage(format!("{}/{}", queue_name, id)))
    }

    async fn create_message(
        &self,
        req: &CreateMessageRequest,
    ) -> Result<Message, CreateMessageError> {
        let id = Uuid::new_v4();
        let mut queues = self.queues.lock().unwrap();
        let content = req.content().clone();
        let message = Message::new(id, content);
        let entry = queues.entry(req.queue_name().clone()).or_insert(Vec::new());
        entry.push(message.clone());
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
