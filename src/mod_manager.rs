use std::ffi::OsString;
use std::fs;
use std::fs::{DirEntry, File};
use std::io::Write;

use anyhow::{Context, Result};
use octocrab::Octocrab;
use reqwest::{Client, ClientBuilder, Url};

struct ModManager {
	octo: Octocrab,
	rewest: Client,
	mod_cache_folder: String,
	current_mods: Vec<DirEntry>,
}

enum ModEntry {
	GitHub {
		owner: String,
		repo: String,
		pattern: String,
	},
	SpTarkov {
		url: Url,
	},
}

struct DownloadReference {
	name: String,
	url: Url,
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
			rewest: ClientBuilder::new().build().unwrap(),
			mod_cache_folder: mod_folder.to_string(),
			current_mods,
		})
	}

	async fn check_newest_release(&self, mod_entry: ModEntry) -> Result<()> {
		let dl_ref = match mod_entry {
			ModEntry::GitHub {
				owner,
				repo,
				pattern,
			} => {
				self.get_newest_github_release(&owner, &repo, &pattern)
					.await?
			}
			ModEntry::SpTarkov { .. } => DownloadReference {
				url: Url::parse("127.0.0.1").unwrap(),
				name: "test".to_string(),
			},
		};

		let string = OsString::from(&dl_ref.name);
		if !self
			.current_mods
			.iter()
			.any(|entry| entry.file_name().eq(&string))
		{
			let response = self.rewest.get(dl_ref.url).send().await?;
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
	) -> Result<DownloadReference> {
		let release = self.octo.repos(owner, repo).releases().get_latest().await?;

		let asset = release
			.assets
			.into_iter()
			.find(|ass| ass.name.contains(assert_pattern))
			.with_context(|| format!("Failed to find assert from pattern: {assert_pattern}"))?;

		Ok(DownloadReference {
			name: asset.name,
			url: asset.browser_download_url,
		})
	}
}
