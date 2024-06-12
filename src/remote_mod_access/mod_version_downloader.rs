use std::cmp::Ordering;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use futures_core::Stream;
use reqwest::Client;
use versions::Versioning;
use crate::remote_mod_access::ModDownloadVersion;
use crate::shared_traits::{ModName, ModVersion, ModVersionDownload};

#[derive(Debug)]
pub struct ModVersionDownloader {
	mod_version: ModDownloadVersion,
	reqwest: Client,
}

impl ModVersionDownload for ModVersionDownloader {
	async fn download(&self) -> anyhow::Result<impl Stream<Item=reqwest::Result<Bytes>>> {
		Ok(self
			.reqwest
			.get(self.mod_version.download_url.clone())
			.send()
			.await?
			.bytes_stream())
	}

	fn get_file_name(&self) -> &str {
		&self.mod_version.file_name
	}

	fn get_upload_date(&self) -> DateTime<Utc> {
		self.mod_version.uploaded_at
	}
}

impl ModVersionDownloader {
	pub(super) fn new(mod_version: ModDownloadVersion, reqwest: &Client) -> Self{
		Self {
			mod_version,
			reqwest: reqwest.clone()
		}
	}
	pub fn mod_version(&self) -> &ModDownloadVersion {
		&self.mod_version
	}
}

impl ModName for ModVersionDownloader {
	fn get_name(&self) -> &str {
		self.mod_version.get_name()
	}

	fn is_same_name<Name: ModName>(&self, mod_name: &Name) -> bool {
		self.mod_version.is_same_name(mod_name)
	}
}

impl ModVersion for ModVersionDownloader {
	fn get_version(&self) -> &Versioning {
		self.mod_version.get_version()
	}

	fn get_order<Version: ModVersion>(&self, mod_version: &Version) -> Ordering {
		self.mod_version.get_order(mod_version)
	}
}
