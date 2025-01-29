use std::fmt::Display;
use std::{fs::File, io::Read, path::PathBuf, slice::Iter, vec::IntoIter};

use serde::Deserialize;
use sha1_smol::Sha1;

use crate::utils::{DateTime, CLIENT_NAME};
use crate::ImmichError;

#[allow(non_snake_case)]
#[derive(Deserialize)]
/// The owner of an [`Asset`] on the Immich server
pub struct Owner {
    id: String,
    email: String,
    name: String,
}

impl Owner {
    /// The id of the owner on the Immich server
    pub fn id(&self) -> &str {
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

impl Display for Owner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} <{}> [{}]", self.name(), self.email(), self.id())
    }
}

#[allow(non_snake_case)]
#[derive(Deserialize)]
/// Album on the remote Immich server
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
pub struct Album {
    albumName: String,
    assetCount: usize,
    id: String,
    owner: Owner,
    shared: bool,
}

impl Album {
    /// The name of the album
    pub fn name(&self) -> &str {
        &self.albumName
    }

    /// The number of images and videos in the album
    pub fn len(&self) -> usize {
        self.assetCount
    }

    /// Returns true if the album does not hold any images or videos
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The unique album id
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The owner of the album
    pub fn owner(&self) -> &Owner {
        &self.owner
    }

    /// Returns true if the album is shared
    pub fn shared(&self) -> bool {
        self.shared
    }
}

#[derive(Deserialize)]
#[serde(transparent)]
/// Container that holds all or some [`Album`]s of the remote Immich server
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
/// let albums = client.albums().unwrap();
/// println!("Number of albums: {}", albums.len());
/// ```
pub struct Albums {
    albums: Vec<Album>,
}

impl Albums {
    /// The number of albums in the container
    pub fn len(&self) -> usize {
        self.albums.len()
    }

    /// Returns true if the container contains no albums
    pub fn is_empty(&self) -> bool {
        self.albums.is_empty()
    }
}

impl IntoIterator for Albums {
    type IntoIter = IntoIter<Album>;
    type Item = Album;
    fn into_iter(self) -> Self::IntoIter {
        self.albums.into_iter()
    }
}

impl<'a> IntoIterator for &'a Albums {
    type IntoIter = Iter<'a, Album>;
    type Item = &'a Album;
    fn into_iter(self) -> Self::IntoIter {
        self.albums.iter()
    }
}

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
    id: String,
    deviceAssetId: String,
    deviceId: String,
    assetData: Vec<u8>,
    owner: Option<Owner>,
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
    /// client.upload(&mut asset).unwrap();
    ///
    /// println!("{}", asset.id());
    /// // "41a3a296-7e86-4eb4-8e44-aead03344fc9"
    /// # }
    /// ```
    pub fn id(&self) -> &str {
        &self.id
    }

    pub(crate) fn id_mut(&mut self) -> &mut String {
        &mut self.id
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
    /// assert_eq!(asset.file_created_at().to_string(), "2025-01-28T05:42:36.000Z");
    /// ```
    pub fn file_created_at(&self) -> &DateTime {
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
    /// assert_eq!(asset.file_modified_at().to_string(), "2025-01-28T05:42:36.000Z");
    /// ```
    pub fn file_modified_at(&self) -> &DateTime {
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
    /// client.upload(&mut asset).unwrap();
    ///
    /// println!("{}", asset.owner().unwrap());
    /// // "Username <email@somewhere> [41a3a296-7e86-4eb4-8e44-aead03344fc9]"
    /// # }
    /// ```
    pub fn owner(&self) -> Option<&Owner> {
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
}

impl Default for Asset {
    fn default() -> Self {
        Self {
            id: "".to_string(),
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
