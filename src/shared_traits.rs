use std::cmp::Ordering;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use futures_core::Stream;
use mockall::automock;
use versions::Versioning;

pub trait ModName {
	fn get_name(&self) -> &str;

	fn is_same_name<Name: ModName>(&self, mod_name: &Name) -> bool;
}

pub trait ModVersion: ModName {
	fn get_version(&self) -> &Versioning;
	fn get_order<Version: ModVersion>(&self, mod_version: &Version) -> Ordering;
}

pub trait ModVersionDownload: ModVersion + Unpin {
	#[allow(async_fn_in_trait)]
	async fn download(&self) -> anyhow::Result<impl Stream<Item=reqwest::Result<Bytes>>>;
	fn get_file_name(&self) -> &str;
	fn get_upload_date(&self) -> DateTime<Utc>;
}

#[automock]
pub trait TimeProvider{
	fn get_current_time(&self) -> DateTime<Utc>;
}