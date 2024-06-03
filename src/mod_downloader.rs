use std::fs::File;
use std::io::Write;
use std::sync::Arc;

use anyhow::Result;
use chrono::{DateTime, Utc};
use reqwest::{Client, ClientBuilder, Url};

use crate::mod_downloader::github_client::GithubClient;
use crate::mod_downloader::spt_client::{SptClient, SptLink};

mod github_client;
mod html_parsers;
mod spt_client;

pub struct ModManager {
	spt_client: SptClient,
	reqwest: Arc<Client>,
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

pub struct ModVersion {
	pub file_name: String,
	pub download_url: Url,
	pub uploaded_at: DateTime<Utc>,
	pub version: String,
}

pub struct ModVersionDownloader {
	mod_version: ModVersion,
	reqwest: Arc<Client>,
}

impl ModVersionDownloader {
	pub async fn download(&self, download_dir: &str) -> Result<()> {
		let response = self
			.reqwest
			.get(self.mod_version.download_url.clone())
			.send()
			.await?;
		let mut result = File::create(format!("{}/{}", download_dir, self.mod_version.file_name))?;
		result.write_all(&response.bytes().await?)?;
		Ok(())
	}
	
	pub fn mod_version(&self) -> &ModVersion{
		&self.mod_version
	}
}

impl ModManager {
	pub fn new() -> Self {
		let client = Arc::new(
			ClientBuilder::new()
				.user_agent("spt_mod_manager_rs")
				.build()
				.unwrap(),
		);
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

		Ok(ModVersionDownloader {
			mod_version,
			reqwest: self.reqwest.clone()
		})
	}
}
