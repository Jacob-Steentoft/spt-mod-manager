use std::cmp::Ordering;
use anyhow::Result;
use chrono::{DateTime, Utc};
use reqwest::{Client, ClientBuilder, Url};
use versions::Versioning;

use crate::mod_downloader::github_client::GithubClient;
use crate::mod_downloader::spt_client::{SptClient, SptLink};
use crate::{ModName, ModVersion};
use crate::mod_downloader::mod_version_downloader::ModVersionDownloader;

mod github_client;
mod html_parsers;
mod spt_client;
mod mod_version_downloader;

pub struct ModDownloader {
	spt_client: SptClient,
	reqwest: Client,
	github: GithubClient,
}

pub enum ModKind {
	GitHub {
		owner: String,
		repo: String,
		pattern: String,
	},
	SpTarkov {
		url: String,
	},
}

#[derive(Debug)]
pub struct ModDownloadVersion {
	pub title: String,
	pub file_name: String,
	pub download_url: Url,
	pub uploaded_at: DateTime<Utc>,
	pub version: Versioning,
}

impl ModName for ModDownloadVersion {
	fn get_name(&self) -> &str {
		self.title.as_str()
	}

	fn is_same_name<Name: ModName>(&self, rhs: &Name) -> bool {
		self.title == rhs.get_name()
	}
}

impl ModVersion for ModDownloadVersion {
	fn get_version(&self) -> &Versioning {
		&self.version
	}
	fn get_order<Version: ModVersion>(&self, rhs: &Version) -> Ordering {
		self.version.cmp(rhs.get_version())
	}
}

impl ModDownloader {
	pub fn new() -> Self {
		let client = 
			ClientBuilder::new()
				.user_agent("spt_mod_manager_rs")
				.build()
				.unwrap();
		Self {
			reqwest: client.clone(),
			spt_client: SptClient::new(client),
			github: GithubClient::new(),
		}
	}

	pub async fn get_newest_release(&self, mod_entry: ModKind) -> Result<ModVersionDownloader> {
		let mod_version = match mod_entry {
			ModKind::GitHub {
				owner,
				repo,
				pattern,
			} => {
				self.github
					.get_newest_github_release(&owner, &repo, &pattern)
					.await?
			}
			ModKind::SpTarkov { url } => {
				let link = SptLink::parse(&url)?;
				self.spt_client.get_latest_version(link).await?
			}
		};

		Ok(ModVersionDownloader::new(mod_version, &self.reqwest))
	}
}
