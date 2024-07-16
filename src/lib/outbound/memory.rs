use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::sync::Mutex;
use uuid::Uuid;

use crate::domain::messages::models::message::{
    CreateMessageError, GetMessageError, QueueListError, QueueSummaryError,
};
use crate::domain::messages::models::message::{
    CreateMessageRequest, GetMessageAction, GetMessageOptions, Message, QueueList, QueueName,
    QueueSummary,
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
        message.set_cursor(self.max_serial);
        self.messages.push_back(message.clone());
        message
    }
}

impl Memory {
    pub async fn new() -> Result<Memory, anyhow::Error> {
        let queues = Arc::new(Mutex::new(HashMap::new()));
        Ok(Self { queues })
    }

    fn get_message(&self, gmo: &GetMessageOptions, queue: &mut Queue, idx: usize) -> Message {
        match gmo.action() {
            GetMessageAction::Browse => queue.messages.get(idx).unwrap().clone(),
            GetMessageAction::Get => queue.messages.remove(idx).unwrap(),
            GetMessageAction::Confirm => queue.messages.remove(idx).unwrap(),
            GetMessageAction::Reserve => {
                let msg = queue.messages.get_mut(idx).unwrap();
                msg.set_reservation(gmo.reservation());
                msg.clone()
            }
            GetMessageAction::Return => {
                let msg = queue.messages.get_mut(idx).unwrap();
                msg.remove_reservation();
                msg.clone()
            }
            GetMessageAction::Query => todo!(),
        }
    }
}

impl MessageRepository for Memory {
    async fn get_message(&self, gmo: GetMessageOptions) -> Result<Message, GetMessageError> {
        let mut queues = self.queues.lock().unwrap();
        let queue = queues
            .get_mut(gmo.queue_name())
            .ok_or(())
            .map_err(|_| GetMessageError::NoMessage(format!("no queue {}", gmo.queue_name())))?;
        let idx = queue
            .messages
            .iter()
            .position(|e| gmo.matches(e))
            .ok_or(())
            .map_err(|_| GetMessageError::NoMessage(format!("{}", gmo.queue_name(),)))?;
        //tracing::info!("removing: {}", remove);

        Ok(self.get_message(&gmo, queue, idx))
    }

    async fn create_message(
        &self,
        queue_name: QueueName,
        req: &CreateMessageRequest,
    ) -> Result<Message, CreateMessageError> {
        let mid = Uuid::new_v4();
        let mut queues = self.queues.lock().unwrap();
        let content = req.content().clone();
        let message = Message::new(mid, None, content);
        let entry = queues.entry(queue_name.clone()).or_default();
        entry.add_message(message.clone());
        Ok(message)
    }

    async fn queue_list(&self) -> Result<QueueList, QueueListError> {
        let queues = self.queues.lock().unwrap();

        Ok(QueueList(
            queues
                .iter()
                .map(|(k, _)| k.to_string())
                .collect::<Vec<_>>(),
        ))
    }

    async fn get_info(&self, gmo: GetMessageOptions) -> Result<QueueSummary, QueueSummaryError> {
        let queues = self.queues.lock().unwrap();
        let queue = queues
            .get(gmo.queue_name())
            .ok_or(())
            .map_err(|_| QueueSummaryError::NoQueue(format!("no queue {}", gmo.queue_name())))?;
        Ok(QueueSummary::new(gmo.queue_name(), queue.messages.len()))
    }
}
