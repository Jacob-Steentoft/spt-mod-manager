use std::cmp::Ordering;
use anyhow::Result;
use chrono::{DateTime, Utc};
use reqwest::{Client, ClientBuilder, Url};
use versions::Versioning;

use crate::mod_downloader::github_client::{GithubClient, GitHubMod};
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
	GitHub(GitHubMod),
	SpTarkov(SptLink),
}

impl ModKind {
	pub fn parse(url: &str, gh_pattern: Option<String>) -> Option<Self>{
		if let Ok(spt_link) = SptLink::parse(url){
			return Some(Self::SpTarkov(spt_link));
		}
		
		if let Some(gh_mod) = gh_pattern.and_then(|t| GitHubMod::parse(url, t).ok()){
			return Some(Self::GitHub(gh_mod));
		}
		None
	}
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
			ModKind::GitHub(gh_mod) => {
				self.github
					.get_newest_github_release(gh_mod)
					.await?
			}
			ModKind::SpTarkov(link)=> {
				self.spt_client.get_latest_version(link).await?
			}
		};

		Ok(ModVersionDownloader::new(mod_version, &self.reqwest))
	}
}
