use crate::remote_mod_access::cache_mod_access::{
	CacheModAccess, CachedModVersion, ModCacheStatus,
};
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use reqwest::{Client, ClientBuilder, Url};
use std::cmp::Ordering;
use serde::{Deserialize, Serialize};
use versions::Versioning;
use crate::path_access::PathAccess;
use crate::remote_mod_access::github_mod_repository::{GITHUB_DOMAIN, GitHubLink, GithubModRepository};
use crate::remote_mod_access::mod_version_downloader::ModVersionDownloader;
use crate::remote_mod_access::spt_mod_repository::{SptModRepository, SptLink, SPT_DOMAIN};
use crate::shared_traits::{ModName, ModVersion};

pub mod cache_mod_access;
mod github_mod_repository;
mod html_parsers;
mod mod_version_downloader;
mod spt_mod_repository;

const SUPPORTED_DOMAINS: &[&str] = &[GITHUB_DOMAIN, SPT_DOMAIN];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
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
	
	pub fn get_supported_domains() -> &'static [&'static str]{
		SUPPORTED_DOMAINS
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
	spt_client: SptModRepository,
	reqwest: Client,
	github: GithubModRepository,
	cache_mod_access: CacheModAccess,
}

impl RemoteModAccess {
	pub async fn init(project: &PathAccess) -> Result<Self> {
		let client = ClientBuilder::new()
			.user_agent("spt_mod_manager_rs")
			.build()
			.unwrap();
		Ok(Self {
			reqwest: client.clone(),
			spt_client: SptModRepository::new(client),
			github: GithubModRepository::new(),
			cache_mod_access: CacheModAccess::init(project).await?,
		})
	}

	pub async fn get_newest_release(&mut self, mod_entry: ModKind) -> Result<CachedModVersion> {
		// TODO: Handle rate limits
		let mod_version = match mod_entry.clone() {
			ModKind::GitHub(gh_mod) => self.github.get_latest_version(gh_mod).await?,
			ModKind::SpTarkov(link) => self.spt_client.get_latest_version(link).await?,
		};

		let cached_mod = match self.cache_mod_access.get_status(&mod_version) {
			ModCacheStatus::SameVersion | ModCacheStatus::NewerVersion => self
				.cache_mod_access
				.get_cached_mod(&mod_version)
				.context("Failed to find cached version")?,
			ModCacheStatus::NotCached | ModCacheStatus::OlderVersion => {
				self.cache_mod_access
					.cache_mod(ModVersionDownloader::new(mod_version, &self.reqwest), mod_entry)
					.await?
			}
		};

		Ok(cached_mod.clone())
	}

	pub async fn get_specific_version(
		&mut self,
		mod_kind: ModKind,
		version: &Versioning,
	) -> Result<Option<CachedModVersion>> {
		// TODO: Handle rate limits
		if let Some(cached_mod) = self.cache_mod_access.get_cached_mod_from_kind(&mod_kind, version) {
			return Ok(Some(cached_mod.clone()))
		};
		
		let mod_version = match mod_kind.clone() {
			ModKind::GitHub(gh_mod) => self.github.get_version(gh_mod, version).await?,
			ModKind::SpTarkov(spt_mod) => self.spt_client.get_version(spt_mod, version).await?,
		};

		let Some(mod_version) = mod_version else {
			return Ok(None);
		};

		let cached_mod = match self.cache_mod_access.get_status(&mod_version) {
			ModCacheStatus::SameVersion => self
				.cache_mod_access
				.get_cached_mod(&mod_version)
				.context("Failed to find cached version")?,
			ModCacheStatus::NewerVersion
			| ModCacheStatus::NotCached
			| ModCacheStatus::OlderVersion => {
				self.cache_mod_access
					.cache_mod(ModVersionDownloader::new(mod_version, &self.reqwest), mod_kind)
					.await?
			}
		};

		Ok(Some(cached_mod.clone()))
	}

	pub async fn remove_cache(&mut self) -> Result<()> {
		self.cache_mod_access.remove_cache().await
	}
}
