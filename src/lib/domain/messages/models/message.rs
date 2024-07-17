use serde::Serialize;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

use derive_more::From;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QueueSummary {
    queue_name: String,
    depth: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GetMessageOptions {
    queue_name: QueueName,
    action: GetMessageAction,
    mid: Option<Uuid>,
    cid: Option<Uuid>,
    reservation: Option<Instant>,
    expiry: Option<Instant>,
    cursor: Option<usize>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GetMessageAction {
    Browse,
    Get,
    Reserve,
    Confirm,
    Return,
    Query,
}

impl TryFrom<&str> for GetMessageAction {
    type Error = GetMessageError;
    fn try_from(value: &str) -> Result<Self, GetMessageError> {
        match value {
            "browse" => Ok(Self::Browse),
            "get" => Ok(Self::Get),
            "reserve" => Ok(Self::Reserve),
            "confirm" => Ok(Self::Confirm),
            "return" => Ok(Self::Return),
            "query" => Ok(Self::Query),
            _ => Err(GetMessageError::InvalidParameter(format!(
                "{} is not valid for action",
                value
            ))),
        }
    }
}

impl GetMessageAction {
    pub fn validate(&self, gmo: &GetMessageOptions) -> Result<(), GetMessageError> {
        match self {
            Self::Reserve => gmo.needs_reservation()?,
            Self::Confirm => gmo.needs_mid()?,
            Self::Return => gmo.needs_mid()?,
            Self::Query => gmo.no_reservation()?,
            Self::Browse => gmo.no_reservation()?,
            _ => {}
        }
        Ok(())
    }
}

impl GetMessageOptions {
    pub fn queue_name(&self) -> &QueueName {
        &self.queue_name
    }
    pub fn action(&self) -> GetMessageAction {
        self.action
    }
    pub fn mid(&self) -> Option<Uuid> {
        self.mid
    }
    pub fn cid(&self) -> Option<Uuid> {
        self.cid
    }
    pub fn reservation(&self) -> &Option<Instant> {
        &self.reservation
    }
    pub fn expiry(&self) -> &Option<Instant> {
        &self.expiry
    }
    pub fn cursor(&self) -> &Option<usize> {
        &self.cursor
    }

    pub fn needs_mid(&self) -> Result<(), GetMessageError> {
        self.mid
            .ok_or(GetMessageError::MissingParameter("id".to_string()))
            .map(|_| ())
    }

    pub fn no_reservation(&self) -> Result<(), GetMessageError> {
        if self.reservation.is_some() {
            Err(GetMessageError::InvalidParameter(
                "reservation_seconds".to_string(),
            ))
        } else {
            Ok(())
        }
    }

    pub fn needs_reservation(&self) -> Result<(), GetMessageError> {
        self.reservation
            .ok_or(GetMessageError::MissingParameter(
                "reservation_seconds".to_string(),
            ))
            .map(|_| ())
    }

    pub fn matches(&self, msg: &Message) -> bool {
        match self.action() {
            GetMessageAction::Browse => {
                !msg.is_reserved()
                    && !msg.is_expired()
                    && (self.mid.is_none() || msg.mid == self.mid.unwrap())
                    && (self.cid.is_none() || msg.mid == self.cid.unwrap())
            }
            GetMessageAction::Get => {
                !msg.is_reserved()
                    && !msg.is_expired()
                    && (self.mid.is_none() || msg.mid == self.mid.unwrap())
                    && (self.cid.is_none() || msg.mid == self.cid.unwrap())
            }
            GetMessageAction::Confirm => msg.is_reserved() && msg.mid == self.mid.unwrap(),
            GetMessageAction::Reserve => {
                !msg.is_reserved()
                    && !msg.is_expired()
                    && (self.mid.is_none() || msg.mid == self.mid.unwrap())
                    && (self.cid.is_none() || msg.mid == self.cid.unwrap())
            }
            GetMessageAction::Return => msg.is_reserved() && msg.mid == self.mid.unwrap(),
            GetMessageAction::Query => todo!(),
        }
    }
}

impl TryFrom<HashMap<String, String>> for GetMessageOptions {
    type Error = GetMessageError;
    fn try_from(m: HashMap<String, String>) -> Result<Self, Self::Error> {
        let queue_name: QueueName = m
            .get("queue_name")
            .ok_or(GetMessageError::MissingParameter("queue_name".to_string()))?
            .try_into()
            .map_err(|_| GetMessageError::InvalidParameter("queue_name".to_string()))?;
        let action = m
            .get("action")
            .ok_or(GetMessageError::MissingParameter("action".to_string()))?
            .as_str()
            .try_into()?;

        let mid = match m.get("mid") {
            None => None,
            Some(s) => Some(
                Uuid::try_parse(s)
                    .map_err(|_| GetMessageError::InvalidParameter("mid".to_string()))?,
            ),
        };
        let cid = match m.get("cid") {
            None => None,
            Some(s) => Some(
                Uuid::try_parse(s)
                    .map_err(|_| GetMessageError::InvalidParameter("mid".to_string()))?,
            ),
        };
        let reservation = match m.get("reservation_seconds") {
            None => None,
            Some(s) => Some(
                s.parse::<u64>()
                    .map(|i| Instant::now() + Duration::from_secs(i))
                    .map_err(|_| {
                        GetMessageError::InvalidParameter("reservation_seconds".to_string())
                    })?,
            ),
        };
        let expiry = match m.get("expiry_seconds") {
            None => None,
            Some(s) => Some(
                s.parse::<u64>()
                    .map(|i| Instant::now() + Duration::from_secs(i))
                    .map_err(|_| {
                        GetMessageError::InvalidParameter("reservation_seconds".to_string())
                    })?,
            ),
        };
        let cursor = match m.get("cursor") {
            None => None,
            Some(s) => Some(
                s.parse()
                    .map_err(|_| GetMessageError::InvalidParameter("cursor".to_string()))?,
            ),
        };
        let gmo = Self {
            queue_name,
            action,
            mid,
            cid,
            reservation,
            expiry,
            cursor,
        };
        action.validate(&gmo)?;
        Ok(gmo)
    }
}

impl QueueSummary {
    pub fn new(queue_name: &QueueName, depth: usize) -> Self {
        Self {
            queue_name: queue_name.to_string(),
            depth,
        }
    }

    pub fn queue_name(&self) -> &String {
        &self.queue_name
    }

    pub fn depth(&self) -> usize {
        self.depth
    }
}

#[derive(Clone, Debug, Error)]
pub enum QueueSummaryError {
    #[error(transparent)]
    Unknown(Arc<anyhow::Error>),
    NoQueue(String),
}

impl From<anyhow::Error> for QueueSummaryError {
    fn from(value: anyhow::Error) -> Self {
        Self::Unknown(Arc::new(value))
    }
}
impl Display for QueueSummaryError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("QueueSummaryError")
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Message {
    mid: uuid::Uuid,
    cid: Option<uuid::Uuid>,
    cursor: usize,
    content: String,
    reservation: Reservation,
    expiry: Expiry,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Reservation {
    Unreserved,
    Until(Instant),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Expiry {
    Permanent,
    Expire(Instant),
}

impl From<Option<Instant>> for Reservation {
    fn from(i: Option<Instant>) -> Reservation {
        match i {
            None => Reservation::Unreserved,
            Some(i) => Reservation::Until(i),
        }
    }
}

impl Message {
    pub fn new(mid: uuid::Uuid, cid: Option<uuid::Uuid>, content: String) -> Self {
        Self {
            mid,
            cid,
            content,
            cursor: 0,
            reservation: Reservation::Unreserved,
            expiry: Expiry::Permanent,
        }
    }

    pub fn mid(&self) -> &uuid::Uuid {
        &self.mid
    }
    pub fn cid(&self) -> Option<&uuid::Uuid> {
        self.cid.as_ref()
    }

    pub fn content(&self) -> &String {
        &self.content
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }
    pub fn set_cursor(&mut self, cursor: usize) {
        self.cursor = cursor
    }

    pub fn is_reserved(&self) -> bool {
        match self.reservation {
            Reservation::Unreserved => false,
            Reservation::Until(inst) => Instant::now() < inst,
        }
    }

    pub fn reserve_for_seconds(&mut self, seconds: u64) {
        self.reservation = Reservation::Until(Instant::now() + Duration::from_secs(seconds))
    }

    pub fn set_reservation(&mut self, inst: &Option<Instant>) {
        if let Some(i) = *inst {
            let new_inst = i;
            self.reservation = Reservation::Until(new_inst)
        }
    }
    pub fn remove_reservation(&mut self) {
        self.reservation = Reservation::Unreserved
    }
    pub fn is_expired(&self) -> bool {
        match self.expiry {
            Expiry::Permanent => false,
            Expiry::Expire(inst) => Instant::now() >= inst,
        }
    }
    pub fn set_expiry(&mut self, inst: &Option<Instant>) {
        if let Some(i) = *inst {
            let new_inst = i;
            self.expiry = Expiry::Expire(new_inst)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QueueName(String);
impl TryFrom<String> for QueueName {
    type Error = QueueNameEmptyError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            Err(QueueNameEmptyError)
        } else {
            Ok(Self(trimmed.to_string()))
        }
    }
}
impl TryFrom<&String> for QueueName {
    type Error = QueueNameEmptyError;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            Err(QueueNameEmptyError)
        } else {
            Ok(Self(trimmed.to_string()))
        }
    }
}

#[derive(Clone, Debug, Error)]
#[error("queue name cannot be empty")]
pub struct QueueNameEmptyError;

impl Display for QueueName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, From)]
pub struct QueueList(pub Vec<String>);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, From)]
pub struct CreateMessageRequest {
    content: String,
    cid: Option<uuid::Uuid>,
}

impl CreateMessageRequest {
    pub fn new(content: String, cid: Option<uuid::Uuid>) -> Self {
        Self { cid, content }
    }

    pub fn cid(&self) -> Option<&uuid::Uuid> {
        self.cid.as_ref()
    }

    pub fn content(&self) -> &String {
        &self.content
    }
}

#[derive(Clone, Debug, Error)]
pub enum CreateMessageError {
    BadQueue(String),
    #[error(transparent)]
    Unknown(Arc<anyhow::Error>),
}
impl From<anyhow::Error> for CreateMessageError {
    fn from(value: anyhow::Error) -> Self {
        Self::Unknown(Arc::new(value))
    }
}

impl Display for CreateMessageError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("Create message error")
    }
}

#[derive(Clone, Debug, Error)]
pub enum QueueListError {
    #[error(transparent)]
    Unknown(Arc<anyhow::Error>),
}
impl From<anyhow::Error> for QueueListError {
    fn from(value: anyhow::Error) -> Self {
        Self::Unknown(Arc::new(value))
    }
}

#[derive(Clone, Debug, Error)]
pub enum GetMessageError {
    BadUuid(String),
    NoMessage(String),
    MissingParameter(String),
    InvalidParameter(String),
    Unknown(Arc<anyhow::Error>),
}

impl From<anyhow::Error> for GetMessageError {
    fn from(value: anyhow::Error) -> Self {
        Self::Unknown(Arc::new(value))
    }
}

impl Display for GetMessageError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GetMessageError::BadUuid(e) => f.write_str(e),
            GetMessageError::NoMessage(e) => f.write_str(e),
            GetMessageError::MissingParameter(e) => f.write_str(e),
            GetMessageError::InvalidParameter(e) => f.write_str(e),
            GetMessageError::Unknown(_) => f.write_str("Unknown"),
        }
    }
}
