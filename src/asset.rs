use std::{fs::File, io::Read, path::PathBuf};

use serde::Deserialize;
use sha1_smol::Sha1;
use ureq::Response;

use crate::upload::{Upload, Uploaded};
use crate::utils::{DateTime, Id, User, CLIENT_NAME};
use crate::{Client, ImmichError, ImmichResult};

pub type AssetId = Id;

#[derive(Debug, Deserialize, PartialEq, Eq)]
/// Different types of [`Asset`]
pub enum AssetType {
    #[serde(rename(deserialize = "IMAGE"))]
    Image,
    #[serde(rename(deserialize = "VIDEO"))]
    Video,
    #[serde(rename(deserialize = "AUDIO"))]
    Audio,
    #[serde(rename(deserialize = "OTHER"))]
    Other,
    #[serde(rename(deserialize = "UNKNOWN"))]
    Unknown,
}

impl Default for AssetType {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Deserialize)]
/// The status of the [`Asset`] on the remote Immich server
pub enum AssetRemoteStatus {
    Unknown,
    Present,
    Absent,
}

impl Default for AssetRemoteStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

#[allow(non_snake_case)]
#[derive(Deserialize)]
/// An `Asset` is an image, video, audio or other media item
///
/// # Examples
///
/// ```
/// use immich::Asset;
///
/// let path = std::path::PathBuf::from("./utils/garden.jpg");
/// let mut asset: Asset = Asset::try_from(path).unwrap();
/// assert!(asset.device_asset_id() == "garden.jpg");
/// ```
pub struct Asset {
    id: Id,
    deviceAssetId: String,
    deviceId: String,
    assetData: Vec<u8>,
    owner: Option<User>,
    #[serde(serialize_with = "serialize_timestamp")]
    fileCreatedAt: DateTime,
    #[serde(serialize_with = "serialize_timestamp")]
    fileModifiedAt: DateTime,
    #[serde(rename = "type")]
    asset_type: AssetType,
    #[serde(skip)]
    remote_status: AssetRemoteStatus,
}

impl Asset {
    /// The Immich id of the asset
    ///
    /// This value will only useful after uploading the asset to the Immich server
    /// or gettings assets from the remote server
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use immich::{Asset, Client};
    ///
    /// let mut asset: Asset = Asset::try_from(PathBuf::from("./utils/garden.jpg")).unwrap();
    /// assert_eq!(asset.id(), "");
    ///
    /// # fn nevercalled(mut asset: Asset) {
    /// let client = Client::with_email("https://immich-web-url/api", "email@somewhere", "s3cr3tpassword").unwrap();
    /// asset.upload(&client).unwrap();
    ///
    /// println!("{}", asset.id());
    /// // "41a3a296-7e86-4eb4-8e44-aead03344fc9"
    /// # }
    /// ```
    pub fn id(&self) -> &Id {
        &self.id
    }

    /// The client-id of the asset, usually the filename
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use immich::Asset;
    ///
    /// let mut asset: Asset = Asset::try_from(PathBuf::from("./utils/garden.jpg")).unwrap();
    /// assert_eq!(asset.device_asset_id(), "garden.jpg");
    /// ```
    pub fn device_asset_id(&self) -> &str {
        &self.deviceAssetId
    }

    /// A mutable reference to the client-id of the asset
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use immich::Asset;
    ///
    /// let mut asset: Asset = Asset::try_from(PathBuf::from("./utils/garden.jpg")).unwrap();
    /// *asset.device_asset_id_mut() = "some_other_name.jpg".to_string();
    /// ```
    pub fn device_asset_id_mut(&mut self) -> &mut String {
        &mut self.deviceAssetId
    }

    /// The id of this client
    ///
    /// Defaults to `"Immich-<VERSION> (Rust Client)"`
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use immich::Asset;
    ///
    /// let mut asset: Asset = Asset::try_from(PathBuf::from("./utils/garden.jpg")).unwrap();
    /// assert_eq!(asset.device_id(), "Immich-0.1 (Rust Client)");
    /// ```
    pub fn device_id(&self) -> &str {
        &self.deviceId
    }

    /// Timestamp of the creation time of the asset
    ///
    /// If the asset is derived from a file, the `ctime` attribute is used. If `ctime` cannot
    /// be derived, it will use `3. October 1990 19:00:00`
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use immich::Asset;
    ///
    /// let mut asset: Asset = Asset::try_from(PathBuf::from("./utils/garden.jpg")).unwrap();
    /// assert_eq!(asset.created_at().to_string(), "2025-01-28T05:42:36.000Z");
    /// ```
    pub fn created_at(&self) -> &DateTime {
        &self.fileCreatedAt
    }

    /// Timestamp of the last modification time of the asset
    ///
    /// If the asset is derived from a file, the `mtime` attribute is used. If `mtime` cannot
    /// be derived, it will use `3. October 1990 19:00:00`
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use immich::Asset;
    ///
    /// let mut asset: Asset = Asset::try_from(PathBuf::from("./utils/garden.jpg")).unwrap();
    /// assert_eq!(asset.modified_at().to_string(), "2025-01-28T05:42:36.000Z");
    /// ```
    pub fn modified_at(&self) -> &DateTime {
        &self.fileModifiedAt
    }

    /// The actual media asset's data
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use immich::Asset;
    ///
    /// let mut asset: Asset = Asset::try_from(PathBuf::from("./utils/garden.jpg")).unwrap();
    /// assert_eq!(asset.asset_data().len(), 165012);
    /// ```
    pub fn asset_data(&self) -> &[u8] {
        &self.assetData
    }

    /// The owner of the asset on the Immich server
    ///
    /// This value will only useful after uploading the asset to the Immich server
    /// or gettings assets from the remote server
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use immich::{Asset, Client};
    ///
    /// let mut asset: Asset = Asset::try_from(PathBuf::from("./utils/garden.jpg")).unwrap();
    /// assert!(asset.owner().is_none());
    /// # fn nevercalled(mut asset: Asset) {
    /// let client = Client::with_email("https://immich-web-url/api", "email@somewhere", "s3cr3tpassword").unwrap();
    /// asset.upload(&client).unwrap();
    ///
    /// println!("{}", asset.owner().unwrap());
    /// // "Username <email@somewhere> [41a3a296-7e86-4eb4-8e44-aead03344fc9]"
    /// # }
    /// ```
    pub fn owner(&self) -> Option<&User> {
        self.owner.as_ref()
    }

    /// The [`AssetType`] of the asset
    ///
    /// This can be `Image`, `Video`, `Other`
    ///
    pub fn asset_type(&self) -> &AssetType {
        &self.asset_type
    }

    /// Mutable refernce to the [`AssetType`] of the asset
    ///
    /// This can be `Image`, `Video`, `Other`
    ///
    pub fn asset_type_mut(&mut self) -> &AssetType {
        &self.asset_type
    }

    /// The status of the asset on the remote Immich server
    ///
    /// This value will only useful after uploading the asset to the Immich server
    /// or gettings assets from the remote server
    ///
    pub fn remote_status(&self) -> &AssetRemoteStatus {
        &self.remote_status
    }

    pub(crate) fn remote_status_mut(&mut self) -> &mut AssetRemoteStatus {
        &mut self.remote_status
    }

    /// The SHA1 checksum of the asset
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use immich::Asset;
    ///
    /// let mut asset: Asset = Asset::try_from(PathBuf::from("./utils/garden.jpg")).unwrap();
    /// assert_eq!(asset.checksum(), "4cb6bfc3d436c695b230d50cb5ab1d79eaf32f6e");
    /// ```
    pub fn checksum(&self) -> String {
        Sha1::from(&self.assetData).hexdigest()
    }

    /// Uploads the asset to the Immich remote server
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
    /// let upload_status = asset.upload(&client).unwrap();
    ///
    /// println!(
    ///     "{}: {} [Remote ID: {}]",
    ///     upload_status.device_asset_id(),
    ///     upload_status.status(),
    ///     upload_status.id()
    /// );
    /// ```
    pub fn upload(&mut self, client: &Client) -> ImmichResult<Uploaded> {
        let resp = Upload::post(client, self)?;
        match resp.status() {
            201 | 200 => self.parse_upload(resp),
            other => Err(ImmichError::Status(other, resp.into_string()?)),
        }
    }

    fn parse_upload(&mut self, response: Response) -> ImmichResult<Uploaded> {
        let mut response: Uploaded = response.into_json()?;
        self.remote_status = AssetRemoteStatus::Present;
        self.id = response.id().clone();
        response
            .device_asset_id_mut()
            .push_str(self.device_asset_id());
        Ok(response)
    }
}

impl Default for Asset {
    fn default() -> Self {
        Self {
            id: Id::default(),
            deviceAssetId: format!("{CLIENT_NAME}-empty"),
            deviceId: CLIENT_NAME.to_string(),
            assetData: vec![],
            owner: None,
            fileCreatedAt: DateTime::default(),
            fileModifiedAt: DateTime::default(),
            asset_type: AssetType::Unknown,
            remote_status: AssetRemoteStatus::Unknown,
        }
    }
}

impl TryFrom<PathBuf> for Asset {
    type Error = ImmichError;
    /// Create an [`Asset`] from a file on the local file system
    ///
    /// This method is the preferred way of creating assets client-side.
    ///
    /// It will read the whole file contents into memory, so don't create
    /// hundreds of assets in one go. Use iterators instead.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use immich::Asset;
    ///
    /// let path = PathBuf::from("./utils/garden.jpg");
    /// let mut asset: Asset = Asset::try_from(path).unwrap();
    /// assert!(asset.device_asset_id() == "garden.jpg");
    /// ```
    ///
    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let file = File::open(&path)?;

        let mut asset = Asset::try_from(file)?;

        if let Some(name) = path.file_name() {
            asset.deviceAssetId.clear();
            asset.deviceAssetId.push_str(&name.to_string_lossy());
        } else {
            println!("Unable to get filename. Use timestamp of creation instead");
        }
        Ok(asset)
    }
}

impl TryFrom<File> for Asset {
    type Error = ImmichError;
    /// Create an [`Asset`] from a `File` object
    ///
    /// It will read the whole file contents into memory, so don't create
    /// hundreds of assets in one go. Use iterators instead.
    ///
    /// # Note
    ///
    /// Use this method only, if you do not have access to the local file system.
    /// Consider using [`Asset::try_from::<PathBuf>`]. The Rust `File` object does
    /// not hold a reference to the filename so the library cannot set it properly.
    /// It will build one using the creation timestamp instead
    ///
    /// # Examples
    ///
    /// ```
    /// use std::fs::File;
    /// use immich::Asset;
    ///
    /// let file = File::open("./utils/garden.jpg").unwrap();
    /// let mut asset: Asset = Asset::try_from(file).unwrap();
    /// assert_eq!(asset.device_asset_id(), "Immich-0.1 (Rust Client) - 20250128_054236");
    /// ```
    ///
    fn try_from(mut file: File) -> Result<Self, Self::Error> {
        let mut asset = Asset::default();

        if let Ok(meta) = file.metadata() {
            if let Ok(time) = meta.created() {
                asset.fileCreatedAt = time.into();
            } else {
                println!("Cannot get creation timestamp from file")
            }
            if let Ok(time) = meta.modified() {
                asset.fileModifiedAt = time.into();
            } else {
                println!("Cannot get modified timestamp from file")
            }
        } else {
            println!("Cannot extract creation and modification timestamps from file")
        }
        asset.deviceAssetId = format!("{CLIENT_NAME} - {}", asset.fileCreatedAt.filename());
        let _ = file.read_to_end(&mut asset.assetData)?;
        Ok(asset)
    }
}

