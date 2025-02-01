use std::thread;

use crossbeam_channel::{unbounded, Sender};
use ureq::Request;

use crate::album::Albums;
use crate::api::bulk_check::BulkUploadCheck;
use crate::api::requests::MovedAsset;
use crate::api::upload::{ParallelUpload, Uploaded};
use crate::asset::Asset;
use crate::host::Host;
use crate::url::Url;
use crate::utils::DEFAULT_HEADERS;
use crate::{Album, ImmichError, ImmichResult, User};

use crate::auth::Authenticated;

pub(crate) trait ImmichClient: Sized {
    fn add_default_header(self) -> Self {
        self
    }

    fn auth(self, auth: &Authenticated) -> Self;
}

impl ImmichClient for Request {
    fn add_default_header(self) -> Self {
        let mut request = self;
        for (header, value) in DEFAULT_HEADERS {
            request = request.set(header, value);
        }
        request
    }

    fn auth(self, auth: &Authenticated) -> Self {
        let header = auth.header();
        self.set(header.0, header.1)
    }
}

/// Client to interact with the Immich remote server
#[derive(Debug, Clone)]
pub struct Client {
    url: Url,
    auth: Authenticated,
}

impl Client {
    /// Connect to the Immich server with email and password authentication
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use immich::{Asset, Client};
    ///
    /// let client = Client::with_email(
    ///     "https://immich-web-url/api",
    ///     "email@somewhere",
    ///     "s3cr3tpassword"
    /// );
    ///
    /// assert!(client.is_ok());
    /// ```
    pub fn with_email(url: &str, email: &str, password: &str) -> ImmichResult<Self> {
        Host::new(url)?.email(email, password)
    }

    /// Connect to the Immich server with API key based authentication
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use immich::{Asset, Client};
    ///
    /// let client = Client::with_key(
    ///     "https://immich-web-url/api",
    ///     "7F4BR7QLvGzyiqcIszQq1nZdvyqFY955yW9msrqyeD"
    /// );
    ///
    /// assert!(client.is_ok());
    /// ```
    pub fn with_key(url: &str, key: &str) -> ImmichResult<Self> {
        Host::new(url)?.key(key)
    }

    pub(crate) fn new(url: Url, auth: Authenticated) -> Self {
        Self { url, auth }
    }

    pub(crate) fn check_auth(&self) -> bool {
        match self.get("/auth/validateToken").call() {
            Ok(response) => response.status() == 200,
            Err(_) => false,
        }
    }

    pub(crate) fn user(&self) -> ImmichResult<User> {
        match self.get("/users/me").call() {
            Ok(response) => {
                if response.status() == 200 {
                    Ok(response.into_json()?)
                } else {
                    Err(ImmichError::Status(
                        response.status(),
                        response.into_string()?,
                    ))
                }
            }
            Err(err) => Err(ImmichError::Transport(err.to_string())),
        }
    }

    pub(crate) fn get(&self, url: &str) -> Request {
        ureq::get(&self.url.add_path(url))
            .add_default_header()
            .auth(&self.auth)
    }

    pub(crate) fn post(&self, url: &str) -> Request {
        ureq::post(&self.url.add_path(url))
            .add_default_header()
            .auth(&self.auth)
    }

    pub(crate) fn put(&self, url: &str) -> Request {
        ureq::put(&self.url.add_path(url))
            .add_default_header()
            .auth(&self.auth)
    }

    /// Returns a list of all albums on the server
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use immich::{Asset, Client};
    ///
    /// let client = Client::with_email(
    ///     "https://immich-web-url/api",
    ///     "email@somewhere",
    ///     "s3cr3tpassword"
    /// ).unwrap();
    ///
    /// for album in client.albums().unwrap() {
    ///     println!("{}: {} assets", album.name(), album.len());
    /// }
    /// ```
    pub fn albums(&self) -> ImmichResult<Albums> {
        self.get("/albums")
            .call()?
            .into_json()
            .map_err(|err| ImmichError::Io { source: err })
    }

    /// Checks if images or videos are already in the database
    ///
    /// This method can be used to cheaply check if upload of a large set of images or videos is
    /// required. However, it is not that useful, to be honest. If you plan on uploading, just
    /// go ahead and upload - the [`Asset::upload`] method does check if the media is already
    /// present on the server, before it uploads, anyway.
    ///
    /// This method requires that the API's response contains the same number of records as the input data.
    ///
    /// It assumes that the input assets don't have information on `remote_status`, or are fine with having
    /// that data overwritten.
    ///
    /// The method updates the [`Asset::remote_status`] values of the passed assets.
    pub fn bulk_check<I: Iterator<Item = Asset> + ExactSizeIterator>(
        &self,
        assets: &mut I,
    ) -> ImmichResult<()> {
        BulkUploadCheck::post(self, assets)
    }

    /// Uploads many images or videos in parallel
    ///
    /// This method is useful for large collections of media assets, for example for upload a
    /// folder of images and videos.
    /// The upload can happen in parallel to the parsing of the media assets, if you use a
    /// proper iterator.
    ///
    /// This methods blocks until all assets are uploaded. If you want to receive progress upate
    /// you can pass a `crossbeam_channel` that is used to send info about each uploaded asset.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use immich::{Asset, Client};
    ///
    /// let client = Client::with_email(
    ///     "https://immich-web-url/api",
    ///     "email@somewhere",
    ///     "s3cr3tpassword"
    /// ).unwrap();
    ///
    /// let path = "/path/to/folder/with/images or videos";
    ///
    /// let asset_iterator = std::fs::read_dir(path).unwrap()
    ///     .filter_map(|entry| {
    ///         let entry = entry.unwrap();
    ///         let path = entry.path();
    ///             if path.is_dir() {
    ///                 None
    ///             } else {
    ///                 Asset::try_from(path).ok()
    ///             }
    ///     });
    ///
    ///
    /// let result = client.upload(5, asset_iterator, None)
    ///     .expect("Parallel upload works");
    ///
    /// for entry in result {
    ///     println!("{}: {}", entry.device_asset_id(), entry.status());
    ///     // e.g.:
    ///     // `IMG_12345.jpg: Created`
    ///     // `IMG_12346.jpg: Duplicate`
    /// }
    /// ```
    ///
    /// Specify a `crossbeam_channel` to print live progress update of the upload
    ///
    /// ```no_run
    /// use crossbeam_channel::unbounded;
    /// use immich::{Asset, Client};
    ///
    /// let client = Client::with_email(
    ///     "https://immich-web-url/api",
    ///     "email@somewhere",
    ///     "s3cr3tpassword"
    /// ).unwrap();
    ///
    /// let path = "/path/to/folder/with/images or videos";
    ///
    /// let asset_iterator = std::fs::read_dir(path).unwrap()
    ///     .filter_map(|entry| {
    ///         let entry = entry.unwrap();
    ///         let path = entry.path();
    ///             if path.is_dir() {
    ///                 None
    ///             } else {
    ///                 Asset::try_from(path).ok()
    ///             }
    ///     });
    ///
    ///
    /// let (sender, receiver) = unbounded::<immich::upload::Uploaded>();
    /// std::thread::spawn(move || {
    ///     while let Ok(result) = receiver.recv() {
    ///         println!("{}: {}", result.status(), result.device_asset_id())
    ///     }
    /// });
    ///
    /// let result = client.upload(5, asset_iterator, Some(sender))
    ///     .expect("Parallel upload works");
    /// ```
    ///
    pub fn upload<I: Iterator<Item = Asset>>(
        &self,
        upload_concurrency: usize,
        assets: I,
        progress_channel: Option<Sender<Uploaded>>,
    ) -> ImmichResult<Vec<Uploaded>> {
        ParallelUpload::new(upload_concurrency).post(self, assets, progress_channel)
    }

    pub(crate) fn auth(&self) -> &Authenticated {
        &self.auth
    }

    /// Uploads assets and adds them to an album after the upload
    ///
    /// The upload can happen in parallel to the parsing of the media assets, if you use a
    /// proper iterator.
    ///
    /// The `threads` parameters specifies the number of parallel upload threads.
    ///
    /// This methods blocks until all assets are uploaded and added to the album.If you want to
    /// receive progress upate you can specify a `crossbeam_channel` that is used to send info
    /// about each uploaded asset.
    ///
    /// ```no_run
    /// use crossbeam_channel::unbounded;
    /// use immich::{Album, Asset, Client};
    ///
    /// let client = Client::with_email(
    ///     "https://immich-web-url/api",
    ///     "email@somewhere",
    ///     "s3cr3tpassword"
    /// ).unwrap();
    ///
    /// let path = "/path/to/folder/with/images or videos";
    ///
    /// let album = Album::get_or_create(&client, "My Album".to_string()).unwrap();
    ///
    /// let asset_iterator = std::fs::read_dir(path).unwrap()
    ///     .filter_map(|entry| {
    ///         let entry = entry.unwrap();
    ///         let path = entry.path();
    ///             if path.is_dir() {
    ///                 None
    ///             } else {
    ///                 Asset::try_from(path).ok()
    ///             }
    ///     });
    ///
    ///
    /// let (sender, receiver) = unbounded::<immich::upload::Uploaded>();
    /// std::thread::spawn(move || {
    ///     while let Ok(result) = receiver.recv() {
    ///         println!("{}: {}", result.status(), result.device_asset_id())
    ///     }
    /// });
    ///
    /// let result = client.upload_to_album(5, asset_iterator, &album, Some(sender))
    ///     .expect("All assets uploaded and added to album");
    /// ```
    ///
    pub fn upload_to_album<I: Iterator<Item = Asset>>(
        &self,
        upload_concurrency: usize,
        assets: I,
        album: &Album,
        progress_channel: Option<Sender<Uploaded>>,
    ) -> ImmichResult<Vec<MovedAsset>> {
        let results = if let Some(sender) = progress_channel {
            let (proxy_sender, proxy_receiver) = unbounded::<Uploaded>();

            let t = thread::spawn(move || {
                let mut thread_results: Vec<Uploaded> = Vec::new();
                while let Ok(uploaded) = proxy_receiver.recv() {
                    thread_results.push(uploaded.clone());
                    sender
                        .send(uploaded)
                        .expect("The feedback channel must remain open throughout");
                }
                thread_results
            });

            self.upload(upload_concurrency, assets, Some(proxy_sender))?;
            t.join().map_err(|_| ImmichError::Multithread)?
        } else {
            self.upload(upload_concurrency, assets, None)?
        };

        album.add_uploaded(self, results)
    }
}
