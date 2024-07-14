use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::sync::Mutex;
use uuid::Uuid;

use crate::domain::messages::models::message::{
    CreateMessageError, GetMessageError, QueueSummaryError,
};
use crate::domain::messages::models::message::{
    CreateMessageRequest, Message, Parameters, QueueName, QueueSummary,
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
    fn retrieve(&self, params: Parameters) -> Result<Message, GetMessageError> {
        let mut queues = self.queues.lock().unwrap();
        let queue = queues
            .get_mut(params.queue_name())
            .ok_or(())
            .map_err(|_| GetMessageError::NoMessage(format!("no queue {}", params.queue_name())))?;
        let idx = queue
            .messages
            .iter()
            .position(|e| {
                (e.is_available() || (e.is_reserved() && params.id().is_some()))
                    && (params.id().is_none() || Some(*e.id()) == params.id())
            })
            .ok_or(())
            .map_err(|_| {
                GetMessageError::NoMessage(format!(
                    "{}/{}",
                    params.queue_name(),
                    params
                        .id()
                        .map(|u| u.to_string())
                        .unwrap_or("<any>".to_string())
                ))
            })?;
        //tracing::info!("removing: {}", remove);
        let msg = if params.remove() {
            queue.messages.remove(idx).unwrap()
        } else {
            let msg = queue.messages.get_mut(idx).unwrap();
            msg.set_reservation(params.reservation());
            msg.clone()
        };
        Ok(msg)
    }
}

impl MessageRepository for Memory {
    async fn get_message(&self, params: Parameters) -> Result<Message, GetMessageError> {
        params.needs_id()?;
        if !params.remove() {
            return Err(GetMessageError::InvalidParameter("remove".to_string()));
        }
        self.retrieve(params)
    }

    async fn get_next_message(&self, params: Parameters) -> Result<Message, GetMessageError> {
        if !params.remove() {
            return Err(GetMessageError::InvalidParameter("remove".to_string()));
        }
        self.retrieve(params)
    }

    async fn browse_message(&self, params: Parameters) -> Result<Message, GetMessageError> {
        params.needs_id()?;
        if params.remove() {
            return Err(GetMessageError::InvalidParameter("remove".to_string()));
        }
        self.retrieve(params)
    }

    async fn browse_next_message(&self, params: Parameters) -> Result<Message, GetMessageError> {
        if params.remove() {
            return Err(GetMessageError::InvalidParameter("remove".to_string()));
        }
        self.retrieve(params)
    }

    async fn reserve_message(&self, params: Parameters) -> Result<Message, GetMessageError> {
        params.needs_id()?;
        params.needs_reservation()?;
        if params.remove() {
            return Err(GetMessageError::InvalidParameter("remove".to_string()));
        }
        self.retrieve(params)
    }
    async fn reserve_next_message(&self, params: Parameters) -> Result<Message, GetMessageError> {
        params.needs_reservation()?;
        if params.remove() {
            return Err(GetMessageError::InvalidParameter("remove".to_string()));
        }
        self.retrieve(params)
    }
    async fn confirm_message(&self, params: Parameters) -> Result<Message, GetMessageError> {
        params.needs_id()?;
        if !params.remove() {
            return Err(GetMessageError::InvalidParameter("remove".to_string()));
        }
        self.retrieve(params)
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
