use std::future::Future;

use crate::domain::messages::models::message::{
    CreateMessageRequest, GetMessageOptions, Message, QueueList,
};

#[allow(unused_imports)]
use crate::domain::messages::models::message::QueueName;
use crate::domain::messages::models::message::{
    CreateMessageError, GetMessageError, QueueListError,
};

pub trait MessageService: Clone + Send + Sync + 'static {
    fn create_message(
        &self,
        queue_name: QueueName,
        req: &CreateMessageRequest,
    ) -> impl Future<Output = Result<Message, CreateMessageError>> + Send;
    fn get_message(
        &self,
        gmo: GetMessageOptions,
    ) -> impl Future<Output = Result<Message, GetMessageError>> + Send;
    fn queue_list(&self) -> impl Future<Output = Result<QueueList, QueueListError>> + Send;
}

pub trait MessageRepository: Send + Sync + Clone + 'static {
    fn create_message(
        &self,
        queue_name: QueueName,
        req: &CreateMessageRequest,
    ) -> impl Future<Output = Result<Message, CreateMessageError>> + Send;
    fn get_message(
        &self,
        gmo: GetMessageOptions,
    ) -> impl Future<Output = Result<Message, GetMessageError>> + Send;
    fn queue_list(&self) -> impl Future<Output = Result<QueueList, QueueListError>> + Send;
}
