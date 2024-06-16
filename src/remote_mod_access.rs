use std::cmp::Ordering;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use reqwest::{Client, ClientBuilder, Url};
use versions::Versioning;

use crate::remote_mod_access::github_client::{GithubClient, GitHubLink};
use crate::remote_mod_access::spt_client::{SptClient, SptLink};
use crate::remote_mod_access::mod_version_downloader::ModVersionDownloader;
use crate::shared_traits::{ModName, ModVersion};

mod github_client;
mod html_parsers;
mod spt_client;
mod mod_version_downloader;

pub struct RemoteModAccess {
	spt_client: SptClient,
	reqwest: Client,
	github: GithubClient,
}

pub enum ModKind {
	GitHub(GitHubLink),
	SpTarkov(SptLink),
}

impl ModKind {
	pub fn parse<S: AsRef<str>>(url: S, gh_pattern: Option<String>) -> Result<Self>{
		if SptLink::starts_with_host(&url) {
			return Ok(Self::SpTarkov(SptLink::parse(url)?))
		}

		if GitHubLink::starts_with_host(&url) {
			let Some(pattern) = gh_pattern else {
				return Err(anyhow!("No asset pattern was provided for Github"))
			};
			
			return Ok(Self::GitHub(GitHubLink::parse(url, pattern)?))
		}
		Err(anyhow!("Unsupported mod host: {}", url.as_ref()))
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

impl RemoteModAccess {
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
	
	pub async fn get_specific_version(&self, mod_kind: ModKind, version: &Versioning) -> Result<Option<ModVersionDownloader>>{
		let mod_version = match mod_kind {
			ModKind::GitHub(gh_mod) => {
				self.github.get_version(gh_mod, version).await?
			}
			ModKind::SpTarkov(spt_mod) => {
				self.spt_client.get_version(spt_mod, version).await?
			}
		};
		
		let Some(mod_version) = mod_version else {
			return Ok(None)
		};
		Ok(Some(ModVersionDownloader::new(mod_version, &self.reqwest)))
	}
}
