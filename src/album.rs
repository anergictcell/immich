use crate::asset::AssetId;
use crate::User;
use std::{slice::Iter, vec::IntoIter};

use serde::{Deserialize, Serialize};

use crate::api::requests::{AddToAlbum, MovedAsset};
use crate::upload::{Status, Uploaded};
use crate::utils::Id;
use crate::{Client, ImmichError, ImmichResult};

#[allow(non_snake_case)]
#[derive(Deserialize, Serialize)]
/// Album on the remote Immich server
///
/// # Examples
///
/// ```no_run
/// use immich::{Client};
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
    #[serde(skip_serializing)]
    assetCount: usize,
    #[serde(skip_serializing)]
    id: Id,
    #[serde(skip_serializing)]
    owner: User,
    #[serde(skip_serializing)]
    shared: bool,
}

impl Album {
    /// Crates a new album on the Immich server
    ///
    /// # Note
    ///
    /// The method does not check if an album with the same does exist already. Immich allows
    /// multiple albums with the same name. If you want to prevent this, use [`Album::get_or_create`].
    ///
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use immich::{Album, Client};
    ///
    /// let client = Client::with_email(
    ///     "https://immich-web-url/api",
    ///     "email@somewhere",
    ///     "s3cr3tpassword"
    /// ).unwrap();
    ///
    /// let album = Album::new(&client, "My album".to_string()).unwrap();
    /// println!("{}: {}", album.name(), album.id());
    /// ```
    pub fn new(client: &Client, name: String) -> ImmichResult<Self> {
        let user = client.user()?;
        let album = Album {
            albumName: name,
            assetCount: 0,
            id: Id::default(),
            owner: user,
            shared: false,
        };
        let response = client.post("/albums").send_json(album)?;

        if response.status() == 201 {
            Ok(response.into_json()?)
        } else {
            Err(response.into())
        }
    }

    /// Retrieves an album from the server or crates a new album
    ///
    /// # Note
    ///
    /// If multiple albums with the same name exist, it will return the first result.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use immich::{Album, Client};
    ///
    /// let client = Client::with_email(
    ///     "https://immich-web-url/api",
    ///     "email@somewhere",
    ///     "s3cr3tpassword"
    /// ).unwrap();
    ///
    /// let album = Album::get_or_create(&client, "My album".to_string()).unwrap();
    /// println!("{}: {}", album.name(), album.id());
    ///
    /// let album2 = Album::get_or_create(&client, "My album".to_string()).unwrap();
    /// assert_eq!(album.id(), album2.id());
    /// ```
    pub fn get_or_create(client: &Client, name: String) -> ImmichResult<Self> {
        if let Some(album) = client
            .albums()?
            .into_iter()
            .find(|album| album.name() == name)
        {
            Ok(album)
        } else {
            Album::new(client, name)
        }
    }

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
    pub fn id(&self) -> &Id {
        &self.id
    }

    /// The owner of the album
    pub fn owner(&self) -> &User {
        &self.owner
    }

    /// Returns true if the album is shared
    pub fn shared(&self) -> bool {
        self.shared
    }

    /// Add assets to the album
    pub fn add_assets<I: Iterator<Item = AssetId>>(
        &self,
        client: &Client,
        ids: I,
    ) -> ImmichResult<Vec<MovedAsset>> {
        if !self.id.is_safe() {
            return Err(ImmichError::InvalidUrl(
                "Album has an invalid Id".to_string(),
            ));
        }
        let payload: AddToAlbum = ids.into();
        let response = client
            .put(&format!("/albums/{}/assets", self.id))
            .send_json(payload)?;

        if response.status() == 200 {
            Ok(response.into_json()?)
        } else {
            Err(ImmichError::Status(
                response.status(),
                response.into_string()?,
            ))
        }
    }

    pub(crate) fn add_uploaded(
        &self,
        client: &Client,
        results: Vec<Uploaded>,
    ) -> ImmichResult<Vec<MovedAsset>> {
        let iter_success = results.iter().filter_map(|uploaded| {
            if uploaded.status() == &Status::Failure {
                None
            } else {
                Some(uploaded.id().clone())
            }
        });

        let iter_failed = results.iter().filter_map(|uploaded| {
            if uploaded.status() == &Status::Failure {
                Some(MovedAsset::from_failed_upload(uploaded.id().clone()))
            } else {
                None
            }
        });

        // add all successfully uploaded assets to the album
        self.add_assets(client, iter_success)
            .map(|mut movedassets| {
                // add all assets that failed to upload to the results
                movedassets.extend(iter_failed);
                movedassets
            })
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
