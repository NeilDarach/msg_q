use crate::domain::messages::models::message::{
    CreateMessageError, GetMessageError, QueueSummaryError,
};
use crate::domain::messages::models::message::{
    CreateMessageRequest, Message, QueueName, QueueSummary,
};
use crate::domain::messages::ports::{MessageRepository, MessageService};

#[derive(Debug, Clone)]
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
    async fn create_message(
        &self,
        req: &CreateMessageRequest,
    ) -> Result<Message, CreateMessageError> {
        self.repo.create_message(req).await
    }
    async fn get_message(
        &self,
        queue_name: QueueName,
        id: &String,
    ) -> Result<Message, GetMessageError> {
        self.repo.get_message(queue_name, id).await
    }

    async fn queue_summary(
        &self,
        queue_name: Option<QueueName>,
    ) -> Result<Vec<QueueSummary>, QueueSummaryError> {
        self.repo.queue_summary(queue_name).await
    }
}
