use std::collections::HashMap;
use std::fmt::{Display, Formatter};
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
pub struct Parameters {
    queue_name: QueueName,
    remove: bool,
    id: Option<Uuid>,
    reservation: Option<Instant>,
}

impl Parameters {
    pub fn queue_name(&self) -> &QueueName {
        &self.queue_name
    }
    pub fn id(&self) -> Option<Uuid> {
        self.id
    }
    pub fn remove(&self) -> bool {
        self.remove
    }
    pub fn reservation(&self) -> &Option<Instant> {
        &self.reservation
    }
    pub fn needs_id(&self) -> Result<(), GetMessageError> {
        if self.id.is_some() {
            Ok(())
        } else {
            Err(GetMessageError::MissingParameter("id".to_string()))
        }
    }
    pub fn needs_reservation(&self) -> Result<(), GetMessageError> {
        if self.reservation.is_some() {
            Ok(())
        } else {
            Err(GetMessageError::MissingParameter(
                "reservation_seconds".to_string(),
            ))
        }
    }
}
impl TryFrom<HashMap<String, String>> for Parameters {
    type Error = GetMessageError;
    fn try_from(m: HashMap<String, String>) -> Result<Self, Self::Error> {
        let queue_name = m
            .get("queue_name")
            .ok_or(GetMessageError::MissingParameter("queue_name".to_string()))?;
        let queue_name = QueueName::new(queue_name)
            .map_err(|_| GetMessageError::InvalidParameter("queue_name".to_string()))?;
        let remove = match m.get("remove") {
            None => false,
            Some(s) => {
                tracing::info!("remove is {}", s);
                s.parse::<bool>()
                    .map_err(|_| GetMessageError::InvalidParameter("remove".to_string()))?
            }
        };
        let id = match m.get("id") {
            None => None,
            Some(s) => {
                Some(Uuid::parse_str(s).map_err(|_| GetMessageError::BadUuid(s.to_string()))?)
            }
        };
        let reservation_seconds = match m.get("reservation_seconds") {
            None => None,
            Some(s) => Some(s.parse::<u64>().map_err(|_| {
                GetMessageError::InvalidParameter("reservation_seconds".to_string())
            })?),
        };
        let reservation = reservation_seconds.map(|s| Instant::now() + Duration::from_secs(s));
        Ok(Self {
            queue_name,
            remove,
            id,
            reservation,
        })
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

#[derive(Debug, Error)]
pub enum QueueSummaryError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Message {
    id: uuid::Uuid,
    serial: usize,
    content: String,
    reserved: Reservation,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Reservation {
    Unreserved,
    Until(Instant),
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
    pub fn new(id: uuid::Uuid, content: String) -> Self {
        Self {
            id,
            content,
            serial: 0,
            reserved: Reservation::Unreserved,
        }
    }

    pub fn id(&self) -> &uuid::Uuid {
        &self.id
    }

    pub fn content(&self) -> &String {
        &self.content
    }

    pub fn serial(&self) -> usize {
        self.serial
    }
    pub fn set_serial(&mut self, serial: usize) {
        self.serial = serial
    }

    pub fn is_available(&self) -> bool {
        match self.reserved {
            Reservation::Unreserved => true,
            Reservation::Until(inst) => Instant::now() >= inst,
        }
    }

    pub fn is_reserved(&self) -> bool {
        self.reserved != Reservation::Unreserved
    }

    pub fn reserve_for_seconds(&mut self, seconds: u64) {
        self.reserved = Reservation::Until(Instant::now() + Duration::from_secs(seconds))
    }

    pub fn set_reservation(&mut self, inst: &Option<Instant>) {
        if let Some(i) = *inst {
            let new_inst = i;
            self.reserved = Reservation::Until(new_inst)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QueueName(String);

#[derive(Clone, Debug, Error)]
#[error("queue name cannot be empty")]
pub struct QueueNameEmptyError;

impl QueueName {
    pub fn new(raw: &str) -> Result<Self, QueueNameEmptyError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            Err(QueueNameEmptyError)
        } else {
            Ok(Self(trimmed.to_string()))
        }
    }
}

impl Display for QueueName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, From)]
pub struct CreateMessageRequest {
    queue_name: QueueName,
    content: String,
}

impl CreateMessageRequest {
    pub fn new(queue_name: QueueName, content: String) -> Self {
        Self {
            queue_name,
            content,
        }
    }

    pub fn queue_name(&self) -> &QueueName {
        &self.queue_name
    }

    pub fn content(&self) -> &String {
        &self.content
    }
}

#[derive(Debug, Error)]
pub enum CreateMessageError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Error)]
pub enum GetMessageError {
    BadUuid(String),
    NoMessage(String),
    MissingParameter(String),
    InvalidParameter(String),
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
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
