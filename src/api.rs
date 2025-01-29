pub(crate) mod bulk_check;
pub(crate) mod requests;
pub mod upload;

use crate::utils::serialize_timestamp;
use serde::Serialize;

use crate::{asset::Asset, utils::DateTime};

#[allow(non_snake_case)]
#[derive(Serialize)]
pub(crate) struct AssetUpload<'a> {
    deviceAssetId: &'a str,
    deviceId: &'a str,
    #[serde(serialize_with = "serialize_timestamp")]
    fileCreatedAt: &'a DateTime,
    #[serde(serialize_with = "serialize_timestamp")]
    fileModifiedAt: &'a DateTime,
    assetData: &'a [u8],
}

impl<'a> From<&'a Asset> for AssetUpload<'a> {
    fn from(asset: &'a Asset) -> Self {
        Self {
            deviceAssetId: asset.device_asset_id(),
            deviceId: asset.device_id(),
            fileCreatedAt: asset.file_created_at(),
            fileModifiedAt: asset.file_modified_at(),
            assetData: asset.asset_data(),
        }
    }
}
