use std::{fmt::Display, time::SystemTime};

use serde::{Deserialize, Serialize, Serializer};
use thiserror::Error;
use time::{
    format_description::BorrowedFormatItem, macros::format_description, Date, OffsetDateTime, Time,
};

pub(crate) const CLIENT_NAME: &str = "Immich-0.1 (Rust Client)";

pub(crate) const DEFAULT_HEADERS: [(&str, &str); 2] =
    [("Accept", "application/json"), ("User-Agent", CLIENT_NAME)];

const DATETIME_FORMAT: &[BorrowedFormatItem<'static>] =
    format_description!("[year]-[month]-[day]T[hour]:[minute]:[second].000Z");

const DATETIME_FILENAME_FORMAT: &[BorrowedFormatItem<'static>] =
    format_description!("[year][month][day]_[hour][minute][second]");

pub(crate) fn serialize_timestamp<S: Serializer>(date: &DateTime, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&date.to_string())
}

#[derive(Deserialize)]
/// Wrapper for UTC-based timetstamps used in Immich metadata
pub struct DateTime(OffsetDateTime);

impl DateTime {
    pub(crate) fn filename(&self) -> String {
        self.0
            .format(DATETIME_FILENAME_FORMAT)
            .expect("OffsetDateTime is formattable using DATETIME_FILENAME_FORMAT")
    }

    /// Crates a new `Datetime`
    pub fn new(
        year: i32,
        month: u8,
        day: u8,
        hour: u8,
        minute: u8,
        second: u8,
    ) -> ImmichResult<Self> {
        Ok(Self(OffsetDateTime::new_utc(
            Date::from_calendar_date(
                year,
                time::Month::try_from(month).map_err(|_| ImmichError::InvalidDate)?,
                day,
            )
            .map_err(|_| ImmichError::InvalidDate)?,
            Time::from_hms(hour, minute, second).map_err(|_| ImmichError::InvalidDate)?,
        )))
    }
}

impl Default for DateTime {
    /// Returns `1990-10-03 12:00:00`
    fn default() -> Self {
        Self::new(1990, 10, 3, 12, 0, 0).expect("Creating fake time is working")
    }
}

impl Display for DateTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .format(DATETIME_FORMAT)
                .expect("OffsetDateTime is formattable using DATETIME_FORMAT")
        )
    }
}

impl From<SystemTime> for DateTime {
    fn from(time: SystemTime) -> Self {
        Self(time.into())
    }
}

impl From<OffsetDateTime> for DateTime {
    fn from(time: OffsetDateTime) -> Self {
        Self(time)
    }
}

#[derive(Error, Debug)]
/// Error types used in this crate
pub enum ImmichError {
    #[error("Unable to authenticate or authentication expired")]
    /// Unable to authenticate to the server
    Auth,
    #[error("Status: [{0}] {1}")]
    /// HTTP status of a connection failure
    Status(u16, String),
    #[error("Error connecting: {0}")]
    /// Error during HTTP connection
    Transport(String),
    #[error("IO error")]
    /// Error reading from filesystem or input streams
    Io {
        #[from]
        source: std::io::Error,
    },
    #[error("Invalid URL: {0}")]
    /// The URL used for creating a client is invalid
    InvalidUrl(String),
    #[error("Invalid response from server")]
    /// The server sent back an invalid response
    InvalidResponse,
    #[error("Error during multithread process")]
    /// The communication channel between different threads crashed
    Multithread,
    #[error("Invalid date")]
    /// The provided date is not valid
    InvalidDate,
    #[error("Invalid ID")]
    /// The provided ID is not valid
    InvalidId,
    #[error("Unable to read Takeout archive")]
    InvalidTakeoutArchive,
}

impl From<ureq::Error> for ImmichError {
    fn from(value: ureq::Error) -> Self {
        match value {
            ureq::Error::Status(code, resp) => {
                ImmichError::Status(code, resp.status_text().to_string())
            }
            ureq::Error::Transport(transport) => ImmichError::Transport(
                transport
                    .message()
                    .unwrap_or("Unknown connection error")
                    .to_string(),
            ),
        }
    }
}

impl From<ureq::Response> for ImmichError {
    fn from(resp: ureq::Response) -> Self {
        ImmichError::Status(resp.status(), resp.status_text().to_string())
    }
}

impl<T> From<crossbeam_channel::SendError<T>> for ImmichError {
    fn from(_value: crossbeam_channel::SendError<T>) -> Self {
        ImmichError::Multithread
    }
}

impl From<crate::takeout::ParseError> for ImmichError {
    fn from(_err: crate::takeout::ParseError) -> Self {
        Self::InvalidTakeoutArchive
    }
}

pub type ImmichResult<T> = Result<T, ImmichError>;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Id {
    id: String,
}

impl Id {
    /// Poor-man's check that the ID is formatted like a UUID
    /// f0edb589-1312-4161-b41e-0a18f127b3dd
    pub(crate) fn is_safe(&self) -> bool {
        if self.id.len() != 36 {
            return false;
        }
        self.id.chars().enumerate().all(|(idx, c)| {
            c.is_alphanumeric()
                || (idx == 8 && c == '-')
                || (idx == 13 && c == '-')
                || (idx == 18 && c == '-')
                || (idx == 23 && c == '-')
        })
    }
}

impl Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}

impl TryFrom<String> for Id {
    type Error = ImmichError;
    fn try_from(id: String) -> Result<Self, Self::Error> {
        let x = Self { id };
        if x.is_safe() {
            Ok(x)
        } else {
            Err(ImmichError::InvalidId)
        }
    }
}

impl TryFrom<&str> for Id {
    type Error = ImmichError;
    fn try_from(id: &str) -> Result<Self, Self::Error> {
        Self::try_from(id.to_string())
    }
}

impl PartialEq<str> for Id {
    fn eq(&self, other: &str) -> bool {
        self.id == other
    }
}

#[allow(non_snake_case)]
#[derive(Deserialize)]
/// The owner of an [`crate::Asset`] on the Immich server
pub struct User {
    id: Id,
    email: String,
    name: String,
}

impl User {
    /// The id of the owner on the Immich server
    pub fn id(&self) -> &Id {
        &self.id
    }

    /// Login email address of the user
    pub fn email(&self) -> &str {
        &self.email
    }

    /// Username
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Display for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} <{}> [{}]", self.name(), self.email(), self.id())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_uuid() {
        assert!(Id::try_from("f0edb589-1312-4161-b41e-0a18f127b3dd").is_ok());
        assert!(Id::try_from("3fa85f64-5717-4562-b3fc-2c963f66afa6").is_ok());
        assert!(Id::try_from("/3fa85f645717-4562-b3fc-2c963f66afa6").is_err());
        assert!(Id::try_from("3fa85f64.5717-4562-b3fc-2c963f66afa6").is_err());
        assert!(Id::try_from("3fa85f64/5717-4562-b3fc-2c963f66afa6").is_err());
        assert!(Id::try_from("3fa85f[]-5717-4562-b3fc-2c963f66afa6").is_err());
        assert!(Id::try_from("3fa()f64-5717-4562-b3fc-2c963f66afa6").is_err());
        assert!(Id::try_from("3f..5f64-5717-4562-b3fc-2c963f66afa6").is_err());
    }
}
