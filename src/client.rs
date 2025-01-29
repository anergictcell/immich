use crossbeam_channel::Sender;
use ureq::{Request, Response};

use crate::api::bulk_check::BulkUploadCheck;
use crate::api::upload::{ParallelUpload, Upload, Uploaded};
use crate::asset::{Albums, Asset, AssetRemoteStatus};
use crate::host::Host;
use crate::url::Url;
use crate::utils::DEFAULT_HEADERS;
use crate::{ImmichError, ImmichResult};

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
    /// go ahead and upload - the [`Client::upload`] method does check if the media is already
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

    fn parse_upload(&self, asset: &mut Asset, response: Response) -> ImmichResult<Uploaded> {
        let mut response: Uploaded = response.into_json()?;
        *asset.remote_status_mut() = AssetRemoteStatus::Present;
        asset.id_mut().push_str(response.id());
        response
            .device_asset_id_mut()
            .push_str(asset.device_asset_id());
        Ok(response)
    }

    /// Uploads a single image or video
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use immich::{Asset, Client};
    ///
    /// let image = "/path/to/image or video";
    /// let mut asset: Asset = std::path::PathBuf::from(image).try_into().unwrap();
    ///
    /// let client = Client::with_email(
    ///     "https://immich-web-url/api",
    ///     "email@somewhere",
    ///     "s3cr3tpassword"
    /// ).unwrap();
    ///
    /// let upload_status = client.upload(&mut asset).unwrap();
    ///
    /// println!(
    ///     "{}: {} [Remote ID: {}]",
    ///     upload_status.device_asset_id(),
    ///     upload_status.status(),
    ///     upload_status.id()
    /// );
    /// ```
    pub fn upload(&self, asset: &mut Asset) -> ImmichResult<Uploaded> {
        let resp = Upload::post(self, asset)?;
        match resp.status() {
            201 | 200 => self.parse_upload(asset, resp),
            other => Err(ImmichError::Status(other, resp.into_string()?)),
        }
    }

    /// Uploads many images or videos in parallel
    ///
    /// This method is useful for large collections of media assets, for example for upload a
    /// folder of images and videos.
    /// The upload can happen in parallel to the parsing of the media assets, if you use a
    /// proper iterator.
    ///
    /// This method is similar to [`Client::parallel_upload_with_progress`], but does not
    /// provide live updates about the upload progress.
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
    /// let result = client.parallel_upload(5, asset_iterator)
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
    pub fn parallel_upload<I: Iterator<Item = Asset>>(
        &self,
        threads: usize,
        assets: I,
    ) -> ImmichResult<Vec<Uploaded>> {
        ParallelUpload::new(threads).post(self, assets)
    }

    /// Uploads many images or videos in parallel with progress updates
    ///
    /// This method is useful for large collections of media assets, for example for upload a
    /// folder of images and videos.
    /// The upload can happen in parallel to the parsing of the media assets, if you use a
    /// proper iterator.
    ///
    /// This method is similar to [`Client::parallel_upload`], but allows for live progress
    /// updates through a feedback `Channel`.
    ///
    /// # Examples
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
    /// client.parallel_upload_with_progress(5, asset_iterator, sender)
    ///     .expect("Parallel upload works");
    /// ```
    ///
    pub fn parallel_upload_with_progress<I: Iterator<Item = Asset>>(
        &self,
        threads: usize,
        assets: I,
        feedback: Sender<Uploaded>,
    ) -> ImmichResult<()> {
        ParallelUpload::new(threads).post_with_progress(self, assets, feedback)
    }

    pub(crate) fn auth(&self) -> &Authenticated {
        &self.auth
    }
}
