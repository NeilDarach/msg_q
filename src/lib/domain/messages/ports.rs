use std::future::Future;

use crate::domain::messages::models::message::{CreateMessageRequest, Message, QueueSummary};

#[allow(unused_imports)]
use crate::domain::messages::models::message::QueueName;
use crate::domain::messages::models::message::{
    CreateMessageError, GetMessageError, QueueSummaryError,
};

pub trait MessageService: Clone + Send + Sync + 'static {
    fn create_message(
        &self,
        req: &CreateMessageRequest,
    ) -> impl Future<Output = Result<Message, CreateMessageError>> + Send;
    fn get_message(
        &self,
        queue_name: QueueName,
        id: &str,
    ) -> impl Future<Output = Result<Message, GetMessageError>> + Send;
    fn get_next_message(
        &self,
        queue_name: QueueName,
    ) -> impl Future<Output = Result<Message, GetMessageError>> + Send;
    fn browse_message(
        &self,
        queue_name: QueueName,
        id: &str,
    ) -> impl Future<Output = Result<Message, GetMessageError>> + Send;
    fn browse_next_message(
        &self,
        queue_name: QueueName,
    ) -> impl Future<Output = Result<Message, GetMessageError>> + Send;
    fn queue_summary(
        &self,
        queue_name: Option<QueueName>,
    ) -> impl Future<Output = Result<Vec<QueueSummary>, QueueSummaryError>> + Send;
}

pub trait MessageRepository: Send + Sync + Clone + 'static {
    fn create_message(
        &self,
        req: &CreateMessageRequest,
    ) -> impl Future<Output = Result<Message, CreateMessageError>> + Send;
    fn get_message(
        &self,
        queue_name: QueueName,
        id: &str,
    ) -> impl Future<Output = Result<Message, GetMessageError>> + Send;
    fn get_next_message(
        &self,
        queue_name: QueueName,
    ) -> impl Future<Output = Result<Message, GetMessageError>> + Send;
    fn browse_message(
        &self,
        queue_name: QueueName,
        id: &str,
    ) -> impl Future<Output = Result<Message, GetMessageError>> + Send;
    fn browse_next_message(
        &self,
        queue_name: QueueName,
    ) -> impl Future<Output = Result<Message, GetMessageError>> + Send;
    fn queue_summary(
        &self,
        queue_name: Option<QueueName>,
    ) -> impl Future<Output = Result<Vec<QueueSummary>, QueueSummaryError>> + Send;
}
