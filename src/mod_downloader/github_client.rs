use crate::mod_downloader::ModVersion;
use anyhow::Context;
use octocrab::Octocrab;

pub struct GithubClient {
	octo: Octocrab,
}

impl GithubClient {
	pub fn new() -> Self {
		Self {
			octo: Octocrab::default(),
		}
	}
	pub async fn get_newest_github_release(
		&self,
		owner: &str,
		repo: &str,
		assert_pattern: &str,
	) -> anyhow::Result<ModVersion> {
		let release = self.octo.repos(owner, repo).releases().get_latest().await?;

		let asset = release
			.assets
			.into_iter()
			.find(|ass| ass.name.contains(assert_pattern))
			.with_context(|| format!("Failed to find assert from pattern: {assert_pattern}"))?;

		Ok(ModVersion {
			file_name: asset.name,
			download_url: asset.browser_download_url,
			version: release.name.context("Found no name")?,
			uploaded_at: asset.created_at,
		})
	}
}
