use anyhow::{anyhow, Context, Error};
use anyhow::Result;
use octocrab::models::repos::{Asset, Release};
use octocrab::Octocrab;
use versions::Versioning;
use winnow::{Parser, PResult};
use winnow::combinator::opt;
use winnow::stream::AsChar;
use winnow::token::{take, take_till, take_until};

use crate::remote_mod_access::ModDownloadVersion;

pub struct GitHubLink {
	owner: String,
	repo: String,
	asset_pattern: String,
	asset_filter: Option<String>,
}

impl GitHubLink {
	pub fn parse<S: AsRef<str>>(url: S, asset_pattern: String, asset_filter: Option<String>) -> Result<Self> {
		let (owner, repo) = validate_url(url.as_ref()).map_err(|_| anyhow!("Failed to parse"))?;
		Ok(Self {
			owner,
			repo,
			asset_pattern,
			asset_filter
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
		// TODO: Handle rate limits
		let release = self
			.octo
			.repos(&gh_mod.owner, &gh_mod.repo)
			.releases()
			.get_latest()
			.await?;

		let version = release.name.clone().context("Found no name")?;
		let asset = Self::filter_asset(&gh_mod, release)?;

		let version = parse_version(&version).ok().flatten().context("Failed to parse version")?;
		Ok(ModDownloadVersion {
			title: gh_mod.repo,
			file_name: asset.name.clone(),
			download_url: asset.browser_download_url.clone(),
			version,
			uploaded_at: asset.created_at,
		})
	}

	pub async fn get_version(&self, gh_mod: GitHubLink, version: &Versioning) -> Result<Option<ModDownloadVersion>>{
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

		let asset = Self::filter_asset(&gh_mod, release)?;

		Ok(Some(ModDownloadVersion {
			title: gh_mod.repo,
			file_name: asset.name,
			download_url: asset.browser_download_url,
			version: version.clone(),
			uploaded_at: asset.created_at,
		}))
	}

	fn filter_asset(gh_mod: &GitHubLink, release:  Release) -> Result<Asset, Error> {
		if let Some(filter) = &gh_mod.asset_filter{
			return release
				.assets
				.into_iter()
				.find(|ass| ass.name.contains(&gh_mod.asset_pattern) && !ass.name.contains(filter))
				.with_context(|| {
					format!(
						"Failed to find assert from pattern: {}, and filter: {:?}",
						&gh_mod.asset_pattern,
						&gh_mod.asset_filter
					)
				})
		};
		release
			.assets
			.into_iter()
			.find(|ass| ass.name.contains(&gh_mod.asset_pattern))
			.with_context(|| {
				format!(
					"Failed to find assert from pattern: {}, and filter: {:?}",
					&gh_mod.asset_pattern,
					&gh_mod.asset_filter
				)
			})
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

pub fn parse_version(version: &str) -> PResult<Option<Versioning>>{
	let (remainder, _) = take_till(0.., AsChar::is_dec_digit).parse_peek(version)?;
	let version = Versioning::parse(remainder).ok().map(|(_, version)| version);
	Ok(version)
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