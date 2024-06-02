use std::ffi::OsString;
use std::fs;
use std::fs::{DirEntry, File};
use std::io::Write;

use anyhow::{Context, Result};
use octocrab::Octocrab;
use reqwest::{Client, ClientBuilder, Url};
use crate::sp_tarkov::{SptClient, SptLink};

struct ModManager {
	octo: Octocrab,
	reqwest: Client,
	mod_cache_folder: String,
	current_mods: Vec<DirEntry>,
	spt_client: SptClient,
}

enum ModKind {
	GitHub {
		owner: String,
		repo: String,
		pattern: String,
	},
	SpTarkov {
		url: String,
	},
}

pub trait ModVersion {
	fn get_version(&self) -> String;
	async fn download(&self, download_dir: File) -> Result<()>;
}

pub trait ModLink {
	fn parse(url: &str) -> Result<Self> where Self: Sized;
}

pub trait ModClient<ME: ModLink, MV: ModVersion>{
	async fn get_latest_version(&self, link: ME) -> Result<MV>;
	async fn get_specific_version(&self, link: ME, kind: &str) -> Result<Option<MV>>;
}

pub struct GitHubDownloadReference {
	pub name: String,
	pub url: Url,
}

impl ModManager {
	fn new(mod_folder: &str) -> Result<Self> {
		let current_mods: Vec<_> = fs::read_dir(mod_folder)?
			.filter(|entry| {
				entry
					.as_ref()
					.is_ok_and(|entry| entry.file_type().is_ok_and(|ft| ft.is_file()))
			})
			.flatten()
			.collect();

		Ok(Self {
			octo: Octocrab::default(),
			reqwest: ClientBuilder::new().build().unwrap(),
			mod_cache_folder: mod_folder.to_string(),
			current_mods,
			spt_client: SptClient::new(),
		})
	}

	async fn get_newest_release(&self, mod_entry: ModKind) -> Result<()> {
		let dl_ref = match mod_entry {
			ModKind::GitHub {
				owner,
				repo,
				pattern,
			} => {
				self.get_newest_github_release(&owner, &repo, &pattern).await?
			}
			ModKind::SpTarkov { url } => {
				let link = SptLink::parse(&url)?;
				let x = self.spt_client.get_latest_version(link).await?;
				x
			}
		};

		let string = OsString::from(&dl_ref.name);
		if !self
			.current_mods
			.iter()
			.any(|entry| entry.file_name().eq(&string))
		{
			let response = self.reqwest.get(dl_ref.url).send().await?;
			let mut result = File::create(format!("{}/{}", self.mod_cache_folder, dl_ref.name))?;
			result.write_all(&response.bytes().await?)?;
		};

		Ok(())
	}
	async fn get_newest_github_release(
		&self,
		owner: &str,
		repo: &str,
		assert_pattern: &str,
	) -> Result<GitHubDownloadReference> {
		let release = self.octo.repos(owner, repo).releases().get_latest().await?;

		let asset = release
			.assets
			.into_iter()
			.find(|ass| ass.name.contains(assert_pattern))
			.with_context(|| format!("Failed to find assert from pattern: {assert_pattern}"))?;

		Ok(GitHubDownloadReference {
			name: asset.name,
			url: asset.browser_download_url,
		})
	}
}

