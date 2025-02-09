use std::collections::{hash_map::Values, HashMap};

use time::OffsetDateTime;

use crate::takeout::Filename;

pub(crate) struct Media {
    date_taken: Option<OffsetDateTime>,
    name: String,
    edited: bool,
    original: bool,
    albums: Vec<String>,
}

impl Media {
    pub fn from_original(name: String, album: String) -> Self {
        Self {
            date_taken: None,
            name,
            edited: false,
            original: true,
            albums: vec![album],
        }
    }

    pub fn from_edited(name: String, album: String) -> Self {
        Self {
            date_taken: None,
            name,
            edited: true,
            original: false,
            albums: vec![album],
        }
    }

    pub fn from_metadata(name: String, album: String, date_taken: OffsetDateTime) -> Self {
        Self {
            date_taken: Some(date_taken),
            name,
            edited: false,
            original: false,
            albums: vec![album],
        }
    }

    pub fn date_taken(&self) -> Option<OffsetDateTime> {
        self.date_taken
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn original(&self) -> bool {
        self.original
    }

    pub fn edited(&self) -> bool {
        self.edited
    }

    pub fn albums(&self) -> &[String] {
        &self.albums
    }

    pub fn add_album(&mut self, album: &str) {
        self.albums.push(album.to_string());
    }

    pub fn set_date_taken(&mut self, date_taken: OffsetDateTime) {
        self.date_taken = Some(date_taken)
    }

    pub fn add_edited(&mut self) {
        self.edited = true
    }

    pub fn add_original(&mut self) {
        self.original = true
    }
}

#[derive(Default)]
pub(crate) struct MediaStore {
    media: HashMap<String, Media>,
}

impl MediaStore {
    pub fn add_metadata(&mut self, file: &Filename, date_taken: OffsetDateTime) {
        self.media
            .entry(file.name().to_string())
            .and_modify(|entry| {
                entry.set_date_taken(date_taken);
                entry.add_album(file.album());
            })
            .or_insert(Media::from_metadata(
                file.name().to_string(),
                file.album().to_string(),
                date_taken,
            ));
    }

    pub fn add_original(&mut self, file: &Filename) {
        self.media
            .entry(file.name().to_string())
            .and_modify(|entry| {
                entry.add_original();
                entry.add_album(file.album());
            })
            .or_insert(Media::from_original(
                file.name().to_string(),
                file.album().to_string(),
            ));
    }

    pub fn add_edited(&mut self, file: &Filename) {
        self.media
            .entry(file.name().to_string())
            .and_modify(|entry| {
                entry.add_edited();
                entry.add_album(file.album());
            })
            .or_insert(Media::from_edited(
                file.name().to_string(),
                file.album().to_string(),
            ));
    }

    pub fn values(&self) -> Values<'_, String, Media> {
        self.media.values()
    }

    pub fn get(&self, key: &str) -> Option<&Media> {
        self.media.get(key)
    }

    pub fn len(&self) -> usize {
        self.media.len()
    }

    pub fn is_empty(&self) -> bool {
        self.media.is_empty()
    }
}
