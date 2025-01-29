use crate::asset::AssetRemoteStatus;
use crate::ImmichError;
use crate::{asset::Asset, Client, ImmichResult};
use std::iter::zip;

use serde::Serialize;

use serde::Deserialize;
use std::vec::IntoIter;

#[derive(Deserialize)]
enum BulkCheckAction {
    #[serde(rename(deserialize = "accept"))]
    Accept,
    #[serde(rename(deserialize = "reject"))]
    Reject,
}

impl From<BulkCheckAction> for AssetRemoteStatus {
    fn from(value: BulkCheckAction) -> Self {
        match value {
            BulkCheckAction::Accept => Self::Absent,
            BulkCheckAction::Reject => Self::Present,
        }
    }
}

#[derive(Deserialize)]
struct BulkCheckResults {
    results: Vec<BulkCheckResult>,
}

impl BulkCheckResults {
    pub fn len(&self) -> usize {
        self.results.len()
    }
}

impl IntoIterator for BulkCheckResults {
    type IntoIter = IntoIter<BulkCheckResult>;
    type Item = BulkCheckResult;
    fn into_iter(self) -> Self::IntoIter {
        self.results.into_iter()
    }
}

#[derive(Deserialize)]
struct BulkCheckResult {
    pub id: String,
    pub action: BulkCheckAction,
}

impl BulkCheckResult {
    fn id(&self) -> &str {
        &self.id
    }
}

#[derive(Serialize)]
struct BulkCheckRequest {
    id: String,
    checksum: String,
}

impl From<&Asset> for BulkCheckRequest {
    fn from(asset: &Asset) -> Self {
        Self {
            id: asset.id().to_string(),
            checksum: asset.checksum(),
        }
    }
}

impl From<Asset> for BulkCheckRequest {
    fn from(asset: Asset) -> Self {
        Self {
            checksum: asset.checksum(),
            id: asset.id().to_string(),
        }
    }
}

pub(crate) struct BulkUploadCheck {}

impl BulkUploadCheck {
    const URL: &str = "/assets/bulk-upload-check";

    pub fn post<I: Iterator<Item = Asset> + ExactSizeIterator>(
        client: &Client,
        assets: &mut I,
    ) -> ImmichResult<()> {
        let data: Vec<BulkCheckRequest> = assets.map(BulkCheckRequest::from).collect();
        let response = client.post(BulkUploadCheck::URL).send_json(data)?;

        if response.status() != 200 {
            return Err(ImmichError::Status(
                response.status(),
                response.status_text().to_string(),
            ));
        }

        let results: BulkCheckResults = response.into_json()?;

        if assets.len() != results.len() {
            return Err(ImmichError::InvalidResponse);
        }

        for (mut asset, result) in zip(assets, results) {
            if asset.id() == result.id() {
                *asset.remote_status_mut() = result.action.into();
            }
        }
        Ok(())
    }
}
