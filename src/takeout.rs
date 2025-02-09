//! Upload images and videos from Google Photos
//!
//! You can download all your Google Photos images and videos using Google Takeout and then
//! migrate them to your Immich server
//!
//! ```no_run
//! use immich::Client;
//! use immich::takeout::Uploader;
//! use immich::upload::Uploaded;
//! use crossbeam_channel::unbounded;
//!
//! let client = Client::with_email(
//!     "https://immich-web-url/api",
//!     "email@somewhere",
//!     "s3cr3tpassword"
//! ).unwrap();
//!
//! let (result_sender, result_receiver) = unbounded::<Uploaded>();
//! let parallel_uploads = 5;
//!
//! let mut takeout_uploader = Uploader::new("/path/to/takeout/file.tar.gz").unwrap();
//!
//! println!("The takeout archive contains {} images and videos", takeout_uploader.len());
//!
//! let res = takeout_uploader
//!     .upload(
//!         &client,
//!         parallel_uploads,
//!         result_sender,
//!         |_| {
//!             true
//!         }
//!     )
//!     .unwrap();
//!
//! for asset in res {
//!     println!("{}", asset.id())
//! }
//! ```
//!

mod file;
mod media;
mod metadata;
mod upload;

use crate::takeout::media::Media;
use crate::takeout::media::MediaStore;
use flate2::read::GzDecoder;
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::{Error, Read, Seek};

use tar::{Archive, Entries, Entry};
use thiserror::Error;
use time::OffsetDateTime;

use crate::takeout::file::{FileType, Filename};
pub use upload::Uploader;

/// Error types used by the `takeout` submodule
#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Invalid filename (expected path/filename): {0}")]
    FileNameError(String),
    #[error("Invalid filename (expected prefix/path/filename): {0}")]
    FilePathError(String),
    #[error("File is not a photo: {0}")]
    NoPhotoError(String),
    #[error("Invalid type")]
    InvalidType,
    #[error("Unable to parse Metadata: {0}")]
    InvalidMetadata(String),
    #[error("IO error")]
    Io {
        #[from]
        source: Error,
    },
}

type ParseResult<T> = Result<T, ParseError>;

/// Defines how to handle edited files
///
/// The Google Photos Takeout data may contain edited and unedited (original) versions of
/// images and videos that were modified by the user or AI agents.
/// This enum defines how to handle the different edited versions.
#[derive(Debug, PartialEq, Eq)]
pub enum HandleEdited {
    /// If a file has both edited and unedited version, the edited one is exclusively used
    PreferEdited,
    /// If a file has both edited and unedited version, both are used
    UseBoth,
    /// If a file has both edited and unedited version, the unedited one is exclusively used
    PreferOriginal,
}

impl HandleEdited {
    fn use_edited(&self) -> bool {
        self != &HandleEdited::PreferOriginal
    }

    fn use_original(&self) -> bool {
        self != &HandleEdited::PreferEdited
    }
}

/// Handles the Takeout archive
///
/// # Examples
///
/// ```no_run
/// use std::fs::File;
/// use immich::takeout;
/// use immich::takeout::Takeout;
///
/// let file = File::open("path/to/archive.tar.gz").unwrap();
/// let mut archive = Takeout::new(file).unwrap();
///
/// for record in archive.records().unwrap() {
///     let record = record.unwrap();
///     println!("{}", record.name());
/// }
/// ```
pub struct Takeout {
    edited_files: HandleEdited,
    media: MediaStore,
    archive: Archive<GzDecoder<File>>,
}

impl Takeout {
    /// Creates a new Takeout archive
    ///
    /// This method is blocking and will take some time to run.
    /// It iterates through the archive to create an internal representation
    /// of all archived objects.
    /// It does **not** read the archive into memory, so it works well also for very large archives
    ///
    /// This method handles edited files in an opinionated manner: If a file exists as both edited
    /// and unedited, it will prefer the edited one.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::fs::File;
    /// use immich::takeout;
    /// use immich::takeout::Takeout;
    ///
    /// let file = File::open("path/to/archive.tar.gz").unwrap();
    /// let archive = Takeout::new(file).unwrap();
    ///
    /// println!("The archive contains {} images and videos", archive.len());
    /// ```
    pub fn new(file: File) -> ParseResult<Self> {
        Self::with_rules(file, HandleEdited::PreferEdited)
    }

    /// Creates a new Takeout archive with defined rules for edited files
    ///
    /// This method is blocking and will take some time to run.
    /// It iterates through the archive to create an internal representation
    /// of all archived objects.
    /// It does **not** read the archive into memory, so it works well also for very large archives
    ///
    /// Use this method if you want to change the handling of edited files (see [`HandleEdited`] for details)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::fs::File;
    /// use immich::takeout;
    /// use immich::takeout::{HandleEdited, Takeout};
    ///
    /// let file = File::open("path/to/archive.tar.gz").unwrap();
    /// let archive = Takeout::with_rules(file, HandleEdited::UseBoth).unwrap();
    ///
    /// println!("The archive contains {} images and videos", archive.len());
    /// ```
    pub fn with_rules(mut file: File, edited_files: HandleEdited) -> ParseResult<Self> {
        let f = file.try_clone()?;
        let archive = Archive::new(GzDecoder::new(f));
        let media = Self::first_scan(archive, &edited_files)?;
        file.rewind()?;
        Ok(Self {
            edited_files,
            media,
            archive: Archive::new(GzDecoder::new(file)),
        })
    }

    fn first_scan(
        mut archive: Archive<GzDecoder<File>>,
        edited_files: &HandleEdited,
    ) -> ParseResult<MediaStore> {
        let mut media = MediaStore::default();
        for entry in archive.entries()? {
            let mut entry = entry?;

            let filename = Filename::try_from(&entry)?;

            match filename.filetype() {
                FileType::Metadata => {
                    if let Ok(date_taken) = metadata::parse(&mut entry) {
                        media.add_metadata(&filename, date_taken);
                    }
                }
                FileType::Edited => {
                    if edited_files.use_edited() {
                        media.add_edited(&filename);
                    }
                }
                FileType::Original => {
                    media.add_original(&filename);
                }
                _ => {
                    // ignoring unknown filetypes
                }
            }
        }
        Ok(media)
    }

    /// Returns the number of images and videos in the Google Takeout archive
    pub fn len(&self) -> usize {
        self.media.len()
    }

    /// returns `true` if the Google Takeout archive is empty and does not contain any
    /// images or videos
    pub fn is_empty(&self) -> bool {
        self.media.is_empty()
    }

    /// Returns an iterator of [`Record`]
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::fs::File;
    /// use immich::takeout;
    /// use immich::takeout::Takeout;
    ///
    /// let file = File::open("path/to/archive.tar.gz").unwrap();
    /// let mut archive = Takeout::new(file).unwrap();
    ///
    /// for record in archive.records().unwrap() {
    ///     let record = record.unwrap();
    ///     println!("{}", record.name());
    ///     println!("{}", record.date_taken().unwrap());
    /// }
    /// ```
    pub fn records(&mut self) -> ParseResult<Iter<'_>> {
        let iter = self.archive.entries()?;
        Ok(Iter::new(iter, &self.edited_files, &self.media))
    }

    pub fn albums(&self) -> TakeoutAlbums {
        let mut albums = TakeoutAlbums::default();
        for file in self.media.values() {
            for album in file.albums() {
                albums
                    .entry(album)
                    .and_modify(|album_list| album_list.push(file.name()))
                    .or_insert(vec![file.name()]);
            }
        }
        albums
    }
}

#[derive(Default)]
pub struct TakeoutAlbums<'a> {
    inner: HashMap<&'a str, Vec<&'a str>>,
}

impl<'a> TakeoutAlbums<'a> {
    pub fn entry(
        &mut self,
        key: &'a str,
    ) -> std::collections::hash_map::Entry<'_, &'a str, Vec<&'a str>> {
        self.inner.entry(key)
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, &str, std::vec::Vec<&str>> {
        self.inner.iter()
    }
}

impl<'a> IntoIterator for TakeoutAlbums<'a> {
    type Item = (&'a str, Vec<&'a str>);
    type IntoIter = std::collections::hash_map::IntoIter<&'a str, std::vec::Vec<&'a str>>;
    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

/// Iterator of [`Record`]
///
/// This iterator can be created from [`Takeout::records`]
///
/// The iterator is a lazy iterator that does not read the actual underlying file contents.
/// They can be accessed using the `Read` trait.
///
/// # Note
/// `Record` items of the iterator should be consumed directly, before advancing the iterator
/// to the next record, as the underlying `tar` library cannot guarantee out-of-order reading
/// of records.
///
/// ```no_run
/// use std::fs::File;
/// use std::io::Write;
/// use immich::takeout;
/// use immich::takeout::Takeout;
///
/// let file = File::open("path/to/archive.tar.gz").unwrap();
/// let mut archive = Takeout::new(file).unwrap();
///
/// // DON'T DO THIS:
/// let mut iter = archive.records().unwrap();
/// let mut record1 = iter.next().unwrap();
/// let _ = iter.next().unwrap();
/// File::create("./file1.jpg").unwrap().write_all(&mut record1.unwrap().data()).unwrap();
///
/// // INSTEAD DO THIS:
/// for record in archive.records().unwrap() {
///     File::create("./file1.jpg").unwrap().write_all(&record.unwrap().data()).unwrap();
/// }
///
/// // OR THIS
/// archive.records().unwrap().for_each(|record| {
///     File::create("./file1.jpg").unwrap().write_all(&record.unwrap().data()).unwrap();
/// })
/// ```
pub struct Iter<'a> {
    iter: Entries<'a, GzDecoder<File>>,
    edited_files: &'a HandleEdited,
    media: &'a MediaStore,
}

impl<'a> Iter<'a> {
    fn new(
        iter: Entries<'a, GzDecoder<File>>,
        edited_files: &'a HandleEdited,
        media: &'a MediaStore,
    ) -> Self {
        Self {
            iter,
            edited_files,
            media,
        }
    }

    fn edited_exists(&self, filename: &Filename) -> bool {
        self.media
            .get(filename.name())
            .expect("Media must exist")
            .edited()
    }

    fn record(&mut self, entry: Entry<'a, GzDecoder<File>>) -> <Self as Iterator>::Item {
        Record::try_from((self.media, entry))
    }

    fn original(
        &mut self,
        filename: &Filename,
        entry: Entry<'a, GzDecoder<File>>,
    ) -> Option<<Self as Iterator>::Item> {
        if !self.edited_exists(filename) || self.edited_files.use_original() {
            Some(self.record(entry))
        } else {
            self.next()
        }
    }

    fn edited(&mut self, entry: Entry<'a, GzDecoder<File>>) -> Option<<Self as Iterator>::Item> {
        if self.edited_files.use_edited() {
            Some(self.record(entry))
        } else {
            self.next()
        }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = ParseResult<Record<'a>>;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(entry) = self.iter.next() {
            let entry = entry.unwrap();

            let filename = Filename::try_from(&entry).unwrap();

            match filename.filetype() {
                FileType::Metadata => self.next(),
                FileType::Original => self.original(&filename, entry),
                FileType::Edited => self.edited(entry),
                _ => self.next(),
            }
        } else {
            None
        }
    }
}

/// Reference to the actual image or video from the Takeout archive
///
/// The reference is a lazy reference and will only read the actual
/// file contents when needed.
pub struct Record<'a> {
    media: &'a Media,
    entry: Entry<'a, GzDecoder<File>>,
}

impl<'a> Record<'a> {
    fn new(media: &'a Media, entry: Entry<'a, GzDecoder<File>>) -> Self {
        Self { media, entry }
    }

    /// Date and time when the photo or video was taken
    ///
    /// this value is taken from the metadata.json file, if available.
    pub fn date_taken(&self) -> Option<OffsetDateTime> {
        self.media.date_taken()
    }

    /// Date and time when the image or video was last edited
    ///
    /// this value is taken from the tar record's metadata, if it is
    /// newer than the [`Record::date_taken`] timestamp.
    pub fn date_modified(&self) -> Option<OffsetDateTime> {
        if let Some(taken) = self.date_taken() {
            if let Ok(mtime) = self.entry.header().mtime() {
                let ot = OffsetDateTime::from_unix_timestamp(mtime.try_into().unwrap()).unwrap();
                if ot >= taken {
                    return Some(ot);
                }
            }
            Some(taken)
        } else {
            None
        }
    }

    /// File name
    pub fn name(&self) -> &str {
        self.media.name()
    }

    /// Returns true if the image/video is not flagged as edited in the archive
    pub fn original(&self) -> bool {
        self.media.original()
    }

    /// Returns true if the image/video is not flagged as edited in the archive
    pub fn edited(&self) -> bool {
        self.media.edited()
    }

    /// List of albums that the image/video is in
    pub fn albums(&self) -> &[String] {
        self.media.albums()
    }

    /// Actual file contents
    ///
    /// This method uses a blocking reader to read the data from the tar archive.
    /// It should not be used **after** advancing the iterator to the next record.
    pub fn data(&mut self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.entry.size().try_into().unwrap());

        self.entry.read_to_end(&mut bytes).unwrap();
        bytes
    }
}

impl Read for Record<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.entry.read(buf)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        self.entry.read_to_end(buf)
    }
}

impl<'a> TryFrom<(&'a MediaStore, Entry<'a, GzDecoder<File>>)> for Record<'a> {
    type Error = ParseError;
    fn try_from(value: (&'a MediaStore, Entry<'a, GzDecoder<File>>)) -> Result<Self, Self::Error> {
        let entry = value.1;
        let filename = Filename::try_from(&entry)?;
        let media = value.0.get(filename.name()).expect("Media must exist");
        Ok(Self::new(media, entry))
    }
}
