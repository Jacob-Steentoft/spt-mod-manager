use crate::mod_downloader::ModDownloadVersion;
use anyhow::Context;
use octocrab::Octocrab;
use versions::Versioning;

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
	) -> anyhow::Result<ModDownloadVersion> {
		let mod_title = self.octo.repos(owner, repo).get().await?.name;
		let release = self.octo.repos(owner, repo).releases().get_latest().await?;

		let asset = release
			.assets
			.into_iter()
			.find(|ass| ass.name.contains(assert_pattern))
			.with_context(|| format!("Failed to find assert from pattern: {assert_pattern}"))?;

		let version = release.name.context("Found no name")?;
		let option = Versioning::new(&version).context("Couldn't parse version")?;
		Ok(ModDownloadVersion {
			title: mod_title,
			file_name: asset.name,
			download_url: asset.browser_download_url,
			version: option,
			uploaded_at: asset.created_at,
		})
	}
}
