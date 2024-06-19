use anyhow::{anyhow, Context};
use anyhow::Result;
use octocrab::Octocrab;
use versions::Versioning;
use winnow::{Parser, PResult};
use winnow::combinator::opt;
use winnow::token::{take, take_until};

use crate::remote_mod_access::ModDownloadVersion;

pub struct GitHubLink {
	owner: String,
	repo: String,
	assert_pattern: String,
}

impl GitHubLink {
	pub fn parse<S: AsRef<str>>(url: S, assert_pattern: String) -> Result<Self> {
		let (owner, repo) = validate_url(url.as_ref()).map_err(|_| anyhow!("Failed to parse"))?;
		Ok(Self {
			owner,
			repo,
			assert_pattern,
		})
	}

	pub fn starts_with_host<S: AsRef<str>>(url: &S) -> bool {
		url.as_ref().starts_with("https://github.com")
	}
}

pub struct GithubClient {
	octo: Octocrab,
}

impl GithubClient {
	pub fn new() -> Self {
		Self {
			octo: Octocrab::default(),
		}
	}
	pub async fn get_newest_github_release(&self, gh_mod: GitHubLink) -> Result<ModDownloadVersion> {
		let mod_title = self
			.octo
			.repos(&gh_mod.owner, &gh_mod.repo)
			.get()
			.await?
			.name;
		let release = self
			.octo
			.repos(&gh_mod.owner, &gh_mod.repo)
			.releases()
			.get_latest()
			.await?;

		let asset = release
			.assets
			.into_iter()
			.find(|ass| ass.name.contains(&gh_mod.assert_pattern))
			.with_context(|| {
				format!(
					"Failed to find assert from pattern: {}",
					&gh_mod.assert_pattern
				)
			})?;

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

	pub async fn get_version(&self, gh_mod: GitHubLink, version: &Versioning) -> Result<Option<ModDownloadVersion>>{
		let mod_title = self
			.octo
			.repos(&gh_mod.owner, &gh_mod.repo)
			.get()
			.await?
			.name;

		let releases = self
			.octo
			.repos(&gh_mod.owner, &gh_mod.repo)
			.releases()
			.list()
			.send()
			.await?;
		let option = releases.into_iter().find(|r| r.name.as_ref().is_some_and(|str| str.contains(&version.to_string())));
		let Some(release) = option else {
			return Ok(None)
		};

		let asset = release
			.assets
			.into_iter()
			.find(|ass| ass.name.contains(&gh_mod.assert_pattern))
			.with_context(|| {
				format!(
					"Failed to find assert from pattern: {}",
					&gh_mod.assert_pattern
				)
			})?;

		Ok(Some(ModDownloadVersion {
			title: mod_title,
			file_name: asset.name,
			download_url: asset.browser_download_url,
			version: version.clone(),
			uploaded_at: asset.created_at,
		}))
	}
}

fn validate_url(input: &str) -> PResult<(String, String)> {
	let (remainder, _) = "https://github.com/".parse_peek(input)?;
	let (remainder, owner) = take_until(0.., "/").parse_peek(remainder)?;
	let (remainder, _) = take(1usize).parse_peek(remainder)?;
	let (remainder, repo) = opt(take_until(0.., "/")).parse_peek(remainder)?;

	let repo = repo.unwrap_or(remainder);
	
	Ok((owner.to_string(), repo.to_string()))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn longer_github_url_should_parse() {
		let result = validate_url("https://github.com/maxloo2/betterkeys-updated/releases/download/v1.2.3/maxloo2-betterkeys-updated-v1.2.3.zip").unwrap();
		assert_eq!(result, ("maxloo2".to_string(), "betterkeys-updated".to_string()));
	}

	#[test]
	fn incorrect_github_url_should_not_parse() {
		let result = validate_url("https://github.com/maxlo");
		assert!(result.is_err())
	}

	#[test]
	fn short_github_url_should_parse() {
		let result = validate_url("https://github.com/maxloo2/betterkeys-updated").unwrap();
		assert_eq!(result, ("maxloo2".to_string(), "betterkeys-updated".to_string()));
	}
}