use crate::domain::messages::models::message::{
    CreateMessageError, GetMessageError, QueueListError,
};
use crate::domain::messages::models::message::{
    CreateMessageRequest, GetMessageOptions, Message, QueueList, QueueName, QueueSummary,
    QueueSummaryError,
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
        queue_name: QueueName,
        req: &CreateMessageRequest,
    ) -> Result<Message, CreateMessageError> {
        self.repo.create_message(queue_name, req).await
    }

    async fn get_message(&self, gmo: GetMessageOptions) -> Result<Message, GetMessageError> {
        self.repo.get_message(gmo).await
    }
    async fn get_info(&self, gmo: GetMessageOptions) -> Result<QueueSummary, QueueSummaryError> {
        self.repo.get_info(gmo).await
    }

    async fn queue_list(&self) -> Result<QueueList, QueueListError> {
        self.repo.queue_list().await
    }
}
