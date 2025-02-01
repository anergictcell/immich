use serde::{Deserialize, Serialize};

use crate::{
    asset::{Asset, AssetId},
    utils::Id,
};

#[derive(Serialize)]
pub(crate) struct BulkCheck {
    id: String,
    checksum: String,
}

impl From<&Asset> for BulkCheck {
    fn from(asset: &Asset) -> Self {
        Self {
            id: asset.id().to_string(),
            checksum: asset.checksum(),
        }
    }
}

impl From<Asset> for BulkCheck {
    fn from(asset: Asset) -> Self {
        Self {
            checksum: asset.checksum(),
            id: asset.id().to_string(),
        }
    }
}

#[derive(Serialize)]
pub(crate) struct AddToAlbum {
    ids: Vec<Id>,
}

impl<I: Iterator<Item = AssetId>> From<I> for AddToAlbum {
    fn from(ids: I) -> Self {
        Self { ids: ids.collect() }
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub enum AssetMoveError {
    #[serde(rename(deserialize = "duplicate"))]
    Duplicate,
    #[serde(rename(deserialize = "no_permission"))]
    NoPermission,
    #[serde(rename(deserialize = "not_found"))]
    NotFound,
    #[serde(rename(deserialize = "unknown"))]
    Unknown,
    UploadFailed,
}

/// Immich API response for adding an asset to an album
#[derive(Deserialize)]
pub struct MovedAsset {
    error: Option<AssetMoveError>,
    id: Id,
    success: bool,
}

impl MovedAsset {
    pub fn new(id: Id, success: bool) -> Self {
        if success {
            Self {
                error: None,
                id,
                success,
            }
        } else {
            Self {
                error: Some(AssetMoveError::Unknown),
                id,
                success,
            }
        }
    }

    pub(crate) fn from_failed_upload(id: Id) -> Self {
        Self {
            error: Some(AssetMoveError::UploadFailed),
            id,
            success: false,
        }
    }

    pub fn error(&self) -> &Option<AssetMoveError> {
        &self.error
    }

    pub fn id(&self) -> &Id {
        &self.id
    }

    pub fn success(&self) -> bool {
        self.success
    }
}
