use crossbeam_channel::{unbounded, Sender};

use crate::{
    api::requests::MovedAsset,
    takeout::Record,
    upload::{Status, Uploaded},
    utils::Id,
    Album, AssetId, Client, ImmichError, ImmichResult,
};
use std::{collections::HashMap, fs::File, path::Path, thread};

use crate::Asset;

use super::{ParseResult, Takeout};

/// Prepare a Google Takeout archive for uploading to Immich
///
/// # Examples
///
/// ```no_run
/// use immich::Client;
/// use immich::takeout::Uploader;
/// use immich::upload::Uploaded;
/// use crossbeam_channel::unbounded;
///
/// let client = Client::with_email(
///     "https://immich-web-url/api",
///     "email@somewhere",
///     "s3cr3tpassword"
/// ).unwrap();
///
/// let (result_sender, result_receiver) = unbounded::<Uploaded>();
/// let parallel_uploads = 5;
///
/// let mut takeout = Uploader::new("/path/to/takeout/file.tar.gz").unwrap();
///
/// println!("The takeout archive contains {} images and videos", takeout.len());
///
/// let res = takeout
///     .upload(
///         &client,
///         parallel_uploads,
///         result_sender,
///         |_| {
///             true
///         }
///     )
///     .unwrap();
///
/// for asset in res {
///     println!("{}", asset.id())
/// }
/// ```
pub struct Uploader {
    takeout: Takeout,
}

impl Uploader {
    /// Crate a new `Uploader` to prepare a Google Takeout archive for uploading to Immich
    ///
    /// This method will scan the full archive once, so it might take some time to run, depending
    /// on the size of the archive. It does **not** read the archive into memory, so it works well
    /// also for very large archives.
    ///
    /// # Errors
    ///
    /// The method might fail if the underlying Takeout archive cannot be read due to IO errors.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use immich::takeout::Uploader;
    ///
    /// let takeout = Uploader::new("/path/to/takeout/file.tar.gz").unwrap();
    ///
    /// println!("The takeout archive contains {} images and videos", takeout.len());
    /// ```
    pub fn new<P: AsRef<Path>>(path: P) -> ParseResult<Self> {
        let file = File::open(path)?;
        let takeout = Takeout::new(file)?;
        Ok(Self { takeout })
    }

    /// Converts all images and videos of the Google Takeout archive to Immich [`Asset`]s
    ///
    /// # Note
    ///
    /// This method will read the actual image or video data from the archive for every asset
    /// into memory. If you have a large archive, do not try to collect the underlying iterator
    /// into a `Vec`. I recommend to only work with the iterator itself.
    ///
    /// # Errors
    ///
    /// The method might fail if the underlying Takeout archive cannot be read due to IO errors.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use immich::takeout::Uploader;
    ///
    /// let mut takeout = Uploader::new("/path/to/takeout/file.tar.gz").unwrap();
    ///
    /// for asset in takeout.assets().unwrap() {
    ///     println!("{}", asset.device_asset_id());
    /// }
    /// ```
    pub fn assets(&mut self) -> ParseResult<impl Iterator<Item = Asset> + use<'_>> {
        Ok(self.takeout.records()?.filter_map(move |record| {
            if let Ok(record) = record {
                if let Ok(asset) = Asset::try_from(record) {
                    return Some(asset);
                }
            }
            None
        }))
    }

    /// Returns the number of images and videos of the Google Takeout archive
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use immich::takeout::Uploader;
    ///
    /// let takeout = Uploader::new("/path/to/takeout/file.tar.gz").unwrap();
    ///
    /// println!("The takeout archive contains {} images and videos", takeout.len());
    /// ```
    pub fn len(&self) -> usize {
        self.takeout.len()
    }

    /// Returns true if the Google Takeout archive is empty
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use immich::takeout::Uploader;
    ///
    /// let takeout = Uploader::new("/path/to/takeout/file.tar.gz").unwrap();
    ///
    /// println!("The takeout archive contains {} images and videos", takeout.len());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.takeout.is_empty()
    }

    /// Filters images and videos of the Google Takeout archive and converts them to Immich [`Asset`]s
    ///
    /// # Note
    ///
    /// This method will read the actual image or video data from the archive for every filtered
    /// asset into memory. If you have a large archive, do not try to collect the underlying
    /// iterator into a `Vec`. I recommend to only work with the iterator itself.
    ///
    /// # Errors
    ///
    /// The method might fail if the underlying Takeout archive cannot be read due to IO errors.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use immich::takeout::Uploader;
    ///
    /// let mut takeout = Uploader::new("/path/to/takeout/file.tar.gz").unwrap();
    ///
    /// for asset in takeout.filter_assets(|record| record.date_taken().unwrap().year() < 2025).unwrap() {
    ///     println!("{}", asset.device_asset_id());
    /// }
    /// ```
    pub fn filter_assets<F: FnMut(&Record<'_>) -> bool>(
        &mut self,
        mut filter: F,
    ) -> ParseResult<impl Iterator<Item = Asset> + use<'_, F>> {
        Ok(self.takeout.records()?.filter_map(move |record| {
            if let Ok(record) = record {
                if filter(&record) {
                    Some(Asset::try_from(record).unwrap())
                } else {
                    None
                }
            } else {
                None
            }
        }))
    }

    /// Upload all images and videos from the Takeout archive to Immich
    ///
    /// All assets are moved to same albums as they were in in Google Photos.
    /// In addition a new album "Google Takout Import" is created for all assets.
    ///
    /// # Errors
    ///
    /// This method has many different ways to fail:
    ///
    /// - The tar archive can't be read: Returns Error
    /// - The image/video data can't be extracted from the tar archive:
    ///   silently ignored and the image/video is skipped
    /// - The "Google Takeout Import album" can't be created: Fails right away
    /// - Some images/videos can't be uploaded to to network, server, etc failure:
    ///   Ignored and the image/video is skipped, reported as failed [`MovedAsset`]
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use immich::{Asset, Client};
    /// use immich::takeout::Uploader;
    /// use immich::upload::Uploaded;
    /// use crossbeam_channel::unbounded;
    ///
    /// let client = Client::with_email(
    ///     "https://immich-web-url/api",
    ///     "email@somewhere",
    ///     "s3cr3tpassword"
    /// ).unwrap();
    ///
    /// let (result_sender, result_receiver) = unbounded::<Uploaded>();
    /// let parallel_uploads = 5;
    ///
    /// let mut takeout = Uploader::new("/path/to/takeout/file.tar.gz").unwrap();
    ///
    /// let res = takeout
    ///     .upload(
    ///         &client,
    ///         parallel_uploads,
    ///         result_sender,
    ///         |record| {
    ///             record.date_taken().unwrap().year() > 2025
    ///             // or, if you want all records to be uploaded, simply use
    ///             // `true`
    ///         }
    ///     )
    ///     .unwrap();
    ///
    /// for asset in res {
    ///     println!("{}", asset.id())
    /// }
    /// ```
    pub fn upload<F: FnMut(&Record<'_>) -> bool>(
        &mut self,
        client: &Client,
        upload_concurrency: usize,
        progress_channel: Sender<Uploaded>,
        filter_records: F,
    ) -> ImmichResult<Vec<MovedAsset>> {
        let assets = self.filter_assets(filter_records)?;

        let (proxy_sender, proxy_receiver) = unbounded::<Uploaded>();

        let threads = thread::spawn(move || {
            let mut thread_results: Vec<Uploaded> = Vec::new();
            while let Ok(uploaded) = proxy_receiver.recv() {
                thread_results.push(uploaded.clone());
                progress_channel
                    .send(uploaded)
                    .expect("The feedback channel must remain open throughout");
            }
            thread_results
        });

        let album = Album::get_or_create(client, "Google Takout Import".to_string())?;
        client.upload_to_album(upload_concurrency, assets, &album, Some(proxy_sender))?;

        let uploaded = threads.join().map_err(|_| ImmichError::Multithread)?;

        Ok(self.recreate_albums(client, uploaded))
    }

    /// Move the uploaded assets to the same albums they were in at Google Photos
    fn recreate_albums(&self, client: &Client, uploaded: Vec<Uploaded>) -> Vec<MovedAsset> {
        /// Helper function to add assets that failed to be moved to an album to the result data
        fn device_ids_to_moved_asset_failure(
            asset_device_ids: &[&str],
            filename2assetid: &mut HashMap<&str, &Id>,
            moved_assets: &mut Vec<MovedAsset>,
        ) {
            moved_assets.extend(asset_device_ids.iter().filter_map(|id| {
                filename2assetid
                    .get(id)
                    .map(|&id| MovedAsset::new(id.clone(), false))
            }))
        }

        // A lookup between the local (Takeout) filename and the Immich Asset Id
        let mut filename2assetid: HashMap<&str, &AssetId> = HashMap::new();
        for asset in &uploaded {
            if asset.status() == &Status::Created || asset.status() == &Status::Duplicate {
                filename2assetid.insert(asset.device_asset_id(), asset.id());
            }
        }

        let mut moved_assets: Vec<MovedAsset> = Vec::new();
        for (album_name, asset_device_ids) in self.takeout.albums() {
            if let Ok(album) = Album::get_or_create(client, album_name.to_string()) {
                // Iterate Immich Asset IDs of all uploaded assets
                let assets = asset_device_ids
                    .iter()
                    .filter_map(|id| filename2assetid.get(id).map(|&id| id.clone()));

                if let Ok(mut result) = album.add_assets(client, assets) {
                    moved_assets.append(&mut result);
                } else {
                    device_ids_to_moved_asset_failure(
                        &asset_device_ids,
                        &mut filename2assetid,
                        &mut moved_assets,
                    );
                }
            } else {
                // failed to move assets to album, for whatever reason
                device_ids_to_moved_asset_failure(
                    &asset_device_ids,
                    &mut filename2assetid,
                    &mut moved_assets,
                );
            }
        }
        moved_assets
    }
}
