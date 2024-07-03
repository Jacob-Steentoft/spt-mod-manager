use crate::remote_mod_access::cache_mod_access::{
	CacheModAccess, CachedModVersion, ModCacheStatus,
};
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use reqwest::{Client, ClientBuilder, Url};
use std::cmp::Ordering;
use std::ffi::OsStr;
use versions::Versioning;

use crate::remote_mod_access::github_client::{GitHubLink, GithubClient};
use crate::remote_mod_access::mod_version_downloader::ModVersionDownloader;
use crate::remote_mod_access::spt_client::{SptClient, SptLink};
use crate::shared_traits::{ModName, ModVersion};

pub mod cache_mod_access;
mod github_client;
mod html_parsers;
mod mod_version_downloader;
mod spt_client;

pub enum ModKind {
	GitHub(GitHubLink),
	SpTarkov(SptLink),
}

impl ModKind {
	pub fn parse<S: AsRef<str>>(url: S, gh_pattern: Option<String>, gh_filter: Option<String>) -> Result<Self> {
		if SptLink::starts_with_host(&url) {
			return Ok(Self::SpTarkov(SptLink::parse(url)?));
		}

		if GitHubLink::starts_with_host(&url) {
			let Some(pattern) = gh_pattern else {
				return Err(anyhow!("No asset pattern was provided for Github"));
			};

			return Ok(Self::GitHub(GitHubLink::parse(url, pattern, gh_filter)?));
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

pub struct RemoteModAccess {
	spt_client: SptClient,
	reqwest: Client,
	github: GithubClient,
	cache_mod_access: CacheModAccess,
}

impl RemoteModAccess {
	pub fn setup(path: impl AsRef<OsStr>) -> Result<Self> {
		let client = ClientBuilder::new()
			.user_agent("spt_mod_manager_rs")
			.build()
			.unwrap();
		Ok(Self {
			reqwest: client.clone(),
			spt_client: SptClient::new(client),
			github: GithubClient::new(),
			cache_mod_access: CacheModAccess::build(path)?,
		})
	}

	pub async fn get_newest_release(&mut self, mod_entry: ModKind) -> Result<&CachedModVersion> {
		// TODO: Handle rate limits
		let mod_version = match mod_entry {
			ModKind::GitHub(gh_mod) => self.github.get_newest_github_release(gh_mod).await?,
			ModKind::SpTarkov(link) => self.spt_client.get_latest_version(link).await?,
		};

		let cached_mod = match self.cache_mod_access.get_status(&mod_version) {
			ModCacheStatus::SameVersion | ModCacheStatus::NewerVersion => self
				.cache_mod_access
				.get_cached_mod(mod_version.get_version())
				.context("Failed to find cached version")?,
			ModCacheStatus::NotCached | ModCacheStatus::OlderVersion => {
				self.cache_mod_access
					.cache_mod(&ModVersionDownloader::new(mod_version, &self.reqwest))
					.await?
			}
		};

		Ok(cached_mod)
	}

	pub async fn get_specific_version(
		&mut self,
		mod_kind: ModKind,
		version: &Versioning,
	) -> Result<Option<&CachedModVersion>> {
		// TODO: Handle rate limits
		let mod_version = match mod_kind {
			ModKind::GitHub(gh_mod) => self.github.get_version(gh_mod, version).await?,
			ModKind::SpTarkov(spt_mod) => self.spt_client.get_version(spt_mod, version).await?,
		};

		let Some(mod_version) = mod_version else {
			return Ok(None);
		};

		let cached_mod = match self.cache_mod_access.get_status(&mod_version) {
			ModCacheStatus::SameVersion => self
				.cache_mod_access
				.get_cached_mod(mod_version.get_version())
				.context("Failed to find cached version")?,
			ModCacheStatus::NewerVersion
			| ModCacheStatus::NotCached
			| ModCacheStatus::OlderVersion => {
				self.cache_mod_access
					.cache_mod(&ModVersionDownloader::new(mod_version, &self.reqwest))
					.await?
			}
		};

		Ok(Some(cached_mod))
	}

	pub fn remove_cache(&mut self) -> Result<()> {
		self.cache_mod_access.remove_cache()
	}
}
