use std::{fmt::Display, time::SystemTime};

use serde::{Deserialize, Serializer};
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

pub type ImmichResult<T> = Result<T, ImmichError>;
