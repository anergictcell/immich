use std::fmt::Display;
use std::fs::File;
use std::io::Read;
use std::num::ParseIntError;

use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use tar::Entry;
use time::format_description::BorrowedFormatItem;
use time::macros::format_description;
use time::OffsetDateTime;

use crate::takeout::{ParseError, ParseResult};

const DATETIME_FORMAT: &[BorrowedFormatItem<'static>] =
    format_description!("[year]-[month]-[day]T[hour]:[minute]:[second].000Z");

#[derive(Serialize, Deserialize)]
struct PhotoTakenTime {
    timestamp: String,
}

impl Display for PhotoTakenTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ts: i64 = self.timestamp.parse::<i64>().unwrap();
        write!(
            f,
            "{}",
            OffsetDateTime::from_unix_timestamp(ts)
                .unwrap()
                .format(&DATETIME_FORMAT)
                .unwrap()
        )
    }
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize)]
struct Metadata {
    photoTakenTime: PhotoTakenTime,
}

impl Metadata {
    fn timestamp(&self) -> Result<i64, ParseIntError> {
        self.photoTakenTime.timestamp.parse::<i64>()
    }
}

pub(crate) fn parse(entry: &mut Entry<'_, GzDecoder<File>>) -> ParseResult<OffsetDateTime> {
    let mut json = String::with_capacity(entry.size().try_into().unwrap());
    let _ = entry.read_to_string(&mut json)?;

    let meta: Metadata = serde_json::from_str(&json).map_err(|_| {
        ParseError::InvalidMetadata(format!(
            "Can't parse JSON for {}",
            entry.path().expect("Metadata comes with a path").display()
        ))
    })?;

    let date_taken =
        OffsetDateTime::from_unix_timestamp(meta.timestamp().map_err(|_| {
            ParseError::InvalidMetadata("Can't parse Timestamp to i64".to_string())
        })?)
        .map_err(|_| {
            ParseError::InvalidMetadata("Can't create OffsetData from timestamp".to_string())
        })?;

    Ok(date_taken)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_json() {
        let data = r#"
{
  "title": "IMG_20130609_101429.jpg",
  "description": "",
  "imageViews": "26",
  "creationTime": {
    "timestamp": "1400220491",
    "formatted": "May 16, 2014, 6:08:11 AM UTC"
  },
  "photoTakenTime": {
    "timestamp": "1370762069",
    "formatted": "Jun 9, 2013, 7:14:29 AM UTC"
  }
}
"#;
        let p: Metadata = serde_json::from_str(data).unwrap();

        assert_eq!(p.photoTakenTime.timestamp, String::from("1370762069"));
    }
}
