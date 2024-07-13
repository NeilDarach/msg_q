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
        id: &String,
    ) -> impl Future<Output = Result<Message, GetMessageError>> + Send;
    fn queue_summary(
        &self,
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
        id: &String,
    ) -> impl Future<Output = Result<Message, GetMessageError>> + Send;
    fn queue_summary(
        &self,
    ) -> impl Future<Output = Result<Vec<QueueSummary>, QueueSummaryError>> + Send;
}
