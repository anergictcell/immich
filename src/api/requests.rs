use serde::Serialize;

use crate::asset::Asset;

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
