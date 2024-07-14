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
    queues: Arc<Mutex<HashMap<QueueName, Queue>>>,
}

#[derive(Debug, Clone, Eq, PartialEq, Default)]
struct Queue {
    messages: VecDeque<Message>,
    max_serial: usize,
}

impl Queue {
    pub fn add_message(&mut self, mut message: Message) -> Message {
        self.max_serial += 1;
        message.set_serial(self.max_serial);
        self.messages.push_back(message.clone());
        message
    }
}

impl Memory {
    pub async fn new() -> Result<Memory, anyhow::Error> {
        let queues = Arc::new(Mutex::new(HashMap::new()));
        Ok(Self { queues })
    }
}

impl Memory {
    fn retrieve(
        &self,
        queue_name: QueueName,
        id: Option<&str>,
        remove: bool,
    ) -> Result<Message, GetMessageError> {
        let id = match id {
            None => None,
            Some(s) => {
                Some(Uuid::parse_str(s).map_err(|_| GetMessageError::BadUuid(s.to_string()))?)
            }
        };

        let mut queues = self.queues.lock().unwrap();
        let queue = queues
            .get_mut(&queue_name)
            .ok_or(())
            .map_err(|_| GetMessageError::NoMessage(format!("no queue {}", queue_name)))?;
        let idx = queue
            .messages
            .iter()
            .position(|e| id.is_none() || Some(*e.id()) == id)
            .ok_or(())
            .map_err(|_| {
                GetMessageError::NoMessage(format!(
                    "{}/{}",
                    queue_name,
                    id.map(|u| u.to_string()).unwrap_or("<any>".to_string())
                ))
            })?;
        //tracing::info!("removing: {}", remove);
        if remove {
            Ok(queue.messages.remove(idx).unwrap())
        } else {
            Ok(queue.messages.get(idx).cloned().unwrap())
        }
    }
}

impl MessageRepository for Memory {
    async fn get_message(
        &self,
        queue_name: QueueName,
        id: &str,
    ) -> Result<Message, GetMessageError> {
        self.retrieve(queue_name, Some(id), true)
    }

    async fn get_next_message(&self, queue_name: QueueName) -> Result<Message, GetMessageError> {
        self.retrieve(queue_name, None, true)
    }

    async fn browse_message(
        &self,
        queue_name: QueueName,
        id: &str,
    ) -> Result<Message, GetMessageError> {
        self.retrieve(queue_name, Some(id), false)
    }

    async fn browse_next_message(&self, queue_name: QueueName) -> Result<Message, GetMessageError> {
        self.retrieve(queue_name, None, false)
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
        entry.add_message(message.clone());
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
            .map(|(k, v)| QueueSummary::new(k, v.messages.len()))
            .collect::<Vec<_>>())
    }
}
