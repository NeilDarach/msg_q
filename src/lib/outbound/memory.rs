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

    fn get_message_impl(&self, gmo: &GetMessageOptions, queue: &mut Queue, idx: usize) -> Message {
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

        Ok(self.get_message_impl(&gmo, queue, idx))
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
        Ok(entry.add_message(message))
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    macro_rules! gmo {
            ($($arg:tt)*) => {{
            let string = format!($($arg)*);
            let gmo: GetMessageOptions = serde_json::from_str::<HashMap<String, String>>(&string)
            .unwrap()
            .try_into()
            .unwrap();
             gmo
        }}
    }

    async fn put(
        store: &mut Memory,
        queue: &str,
        data: &str,
        cid: Option<&str>,
    ) -> Result<Message, CreateMessageError> {
        let req = CreateMessageRequest::new(
            data.to_string(),
            cid.map(|s| uuid::Uuid::try_parse(s).unwrap()),
        );
        store
            .create_message(queue.to_string().try_into().unwrap(), &req)
            .await
    }

    async fn depth(store: &Memory, queue_name: &str) -> usize {
        let gmo = gmo!(r#"{{"action":"query","queue_name":"{}"}}"#, queue_name);
        let summary = store.get_info(gmo).await.unwrap();
        summary.depth()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_create_store() {
        let store = Memory::new().await;
        assert!(store.is_ok(), "{:?}", store);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_empty_store() {
        let store = Memory::new().await.unwrap();
        let queue_list = store.queue_list().await.unwrap();
        assert!(queue_list.0.is_empty(), "{:?}", queue_list);

        let gmo = gmo!(r#"{{"action":"query","queue_name":"test"}}"#,);
        let summary = store.get_info(gmo).await;
        assert!(summary.is_err(), "{:?}", summary);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_create_message() {
        let store = Memory::new().await.unwrap();
        let req = CreateMessageRequest::new("msg1".to_string(), None);

        let msg1 = store
            .create_message("queue1".to_string().try_into().unwrap(), &req)
            .await
            .unwrap();

        assert_eq!(msg1.content(), &"msg1".to_string());
        assert_eq!(msg1.cursor(), 1);
        let msg2 = store
            .create_message("queue1".to_string().try_into().unwrap(), &req)
            .await
            .unwrap();
        assert_eq!(msg2.cursor(), 2);

        let queue_list = store.queue_list().await.unwrap();
        assert_eq!(queue_list.0.len(), 1, "{:?}", queue_list);

        let gmo = gmo!(r#"{{"action":"query","queue_name":"queue1"}}"#,);
        let summary = store.get_info(gmo).await;
        assert!(summary.is_ok(), "{:?}", summary);
        let summary = summary.unwrap();
        assert_eq!(
            summary,
            QueueSummary::new(&"queue1".to_string().try_into().unwrap(), 2)
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_browse_message() {
        let store = Memory::new().await.unwrap();
        let req = CreateMessageRequest::new("msg1".to_string(), None);
        let msg1 = store
            .create_message("queue1".to_string().try_into().unwrap(), &req.clone())
            .await
            .unwrap();
        let msg2 = store
            .create_message("queue1".to_string().try_into().unwrap(), &req)
            .await
            .unwrap();
        let gmo = gmo!(r#"{{"action":"browse","queue_name":"queue1"}}"#,);
        let msg = store.get_message(gmo.clone()).await;
        assert!(msg.is_ok());
        let msg = msg.unwrap();
        assert_eq!(msg, msg1);

        let msg = store.get_message(gmo.clone()).await;
        assert!(msg.is_ok());
        let msg = msg.unwrap();
        assert_eq!(msg, msg1);

        let gmo = gmo!(
            r#"{{"action":"browse","queue_name":"queue1","mid":"{}"}}"#,
            msg2.mid()
        );
        let msg = store.get_message(gmo.clone()).await;
        assert!(msg.is_ok());
        let msg = msg.unwrap();
        assert_eq!(msg, msg2);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_message() {
        let mut store = Memory::new().await.unwrap();
        let msg1 = put(&mut store, "queue1", "msg1", None).await.unwrap();
        let msg2 = put(&mut store, "queue1", "msg2", None).await.unwrap();
        assert_eq!(depth(&store, "queue1").await, 2);
        let gmo = gmo!(r#"{{"action":"get","queue_name":"queue1"}}"#,);
        let msg = store.get_message(gmo.clone()).await;
        assert!(msg.is_ok());
        let msg = msg.unwrap();
        assert_eq!(msg, msg1);

        let msg = store.get_message(gmo.clone()).await;
        assert!(msg.is_ok());
        let msg = msg.unwrap();
        assert_eq!(msg, msg2);

        assert_eq!(depth(&store, "queue1").await, 0)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_mid_message() {
        let mut store = Memory::new().await.unwrap();
        let _msg1 = put(&mut store, "queue1", "msg1", None).await.unwrap();
        let msg2 = put(&mut store, "queue1", "msg2", None).await.unwrap();
        let _msg3 = put(&mut store, "queue1", "msg3", None).await.unwrap();
        assert_eq!(depth(&store, "queue1").await, 3);
        let gmo = gmo!(
            r#"{{"action":"get","queue_name":"queue1","mid":"{}"}}"#,
            msg2.mid()
        );
        let msg = store.get_message(gmo.clone()).await;
        assert!(msg.is_ok());
        let msg = msg.unwrap();
        assert_eq!(msg, msg2);
        assert_eq!(depth(&store, "queue1").await, 2);
        let msg = store.get_message(gmo.clone()).await;
        assert!(msg.is_err());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_browse_mid_message() {
        let mut store = Memory::new().await.unwrap();
        let _msg1 = put(&mut store, "queue1", "msg1", None).await.unwrap();
        let msg2 = put(&mut store, "queue1", "msg2", None).await.unwrap();
        let _msg3 = put(&mut store, "queue1", "msg3", None).await.unwrap();
        assert_eq!(depth(&store, "queue1").await, 3);
        let gmo = gmo!(
            r#"{{"action":"browse","queue_name":"queue1","mid":"{}"}}"#,
            msg2.mid()
        );
        let msg = store.get_message(gmo.clone()).await;
        assert!(msg.is_ok());
        let msg = msg.unwrap();
        assert_eq!(msg, msg2);
        assert_eq!(depth(&store, "queue1").await, 3);
        let msg = store.get_message(gmo.clone()).await;
        assert!(msg.is_ok());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_confirm_message() {
        let mut store = Memory::new().await.unwrap();
        let _msg1 = put(&mut store, "queue1", "msg1", None).await.unwrap();
        let msg2 = put(&mut store, "queue1", "msg2", None).await.unwrap();
        let _msg3 = put(&mut store, "queue1", "msg3", None).await.unwrap();
        assert_eq!(depth(&store, "queue1").await, 3);
        let gmo = gmo!(
            r#"{{"action":"reserve","queue_name":"queue1","mid":"{}","reservation_seconds":"10"}}"#,
            msg2.mid()
        );
        let msg = store.get_message(gmo.clone()).await;
        assert!(msg.is_ok());
        let msg = msg.unwrap();
        assert_eq!(msg.content(), &"msg2".to_string());
        assert_eq!(depth(&store, "queue1").await, 3);
        let fail = store.get_message(gmo.clone()).await;
        assert!(fail.is_err());
        let gmo = gmo!(
            r#"{{"action":"confirm","queue_name":"queue1","mid":"{}"}}"#,
            msg.mid()
        );
        let msg = store.get_message(gmo.clone()).await;
        assert!(msg.is_ok());
        assert_eq!(depth(&store, "queue1").await, 2);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_return_message() {
        let mut store = Memory::new().await.unwrap();
        let _msg1 = put(&mut store, "queue1", "msg1", None).await.unwrap();
        let msg2 = put(&mut store, "queue1", "msg2", None).await.unwrap();
        let _msg3 = put(&mut store, "queue1", "msg3", None).await.unwrap();
        assert_eq!(depth(&store, "queue1").await, 3);
        let reserve_gmo = gmo!(
            r#"{{"action":"reserve","queue_name":"queue1","mid":"{}","reservation_seconds":"10"}}"#,
            msg2.mid()
        );
        let msg = store.get_message(reserve_gmo.clone()).await;
        assert!(msg.is_ok());
        let msg = msg.unwrap();
        assert_eq!(msg.content(), &"msg2".to_string());
        assert_eq!(depth(&store, "queue1").await, 3);
        let fail = store.get_message(reserve_gmo.clone()).await;
        assert!(fail.is_err());
        let return_gmo = gmo!(
            r#"{{"action":"return","queue_name":"queue1","mid":"{}"}}"#,
            msg.mid()
        );
        let msg = store.get_message(return_gmo.clone()).await;
        assert!(msg.is_ok());
        assert_eq!(depth(&store, "queue1").await, 3);
        let msg = store.get_message(reserve_gmo.clone()).await;
        assert!(msg.is_ok());
        let msg = msg.unwrap();
        assert_eq!(msg.content(), &"msg2".to_string());
    }
}
