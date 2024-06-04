use std::io::{Seek, Write};

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

pub struct ModVersion {
	pub title: String,
	pub file_name: String,
	pub download_url: Url,
	pub uploaded_at: DateTime<Utc>,
	pub version: String,
}

pub struct ModVersionDownloader {
	mod_version: ModVersion,
	reqwest: Client,
}

impl ModVersionDownloader {
	pub async fn download<W: Write + Seek>(&self, download_to: &mut W) -> Result<()> {
		let response = self
			.reqwest
			.get(self.mod_version.download_url.clone())
			.send()
			.await?;

		download_to.write_all(&response.bytes().await?)?;
		Ok(())
	}
	
	pub fn mod_version(&self) -> &ModVersion{
		&self.mod_version
	}
}

impl ModManager {
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

		Ok(ModVersionDownloader {
			mod_version,
			reqwest: self.reqwest.clone()
		})
	}
}
