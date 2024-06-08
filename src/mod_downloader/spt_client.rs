use anyhow::{anyhow, Context, Result};
use reqwest::{Client, Url};
use versions::Versioning;
use winnow::ascii::digit1;
use winnow::combinator::repeat;
use winnow::prelude::*;
use winnow::token::{take, take_until};

use crate::mod_downloader::html_parsers::SptMod;
use crate::mod_downloader::{html_parsers, ModDownloadVersion};

pub struct SptClient {
	client: Client,
}

impl SptClient {
	pub fn new(client: Client) -> Self {
		Self { client }
	}

	pub async fn get_latest_version(&self, spt_link: SptLink) -> Result<ModDownloadVersion> {
		let spt_mod = self.get_all_versions(spt_link).await?;
		let mod_version = spt_mod
			.versions
			.into_iter()
			.max_by(|x, x1| x.version.cmp(&x1.version))
			.context("Found no mods")?;

		let download_url = self.get_mod_dl_link(mod_version.download_url).await?;

		let file_name = get_mod_filename(download_url.as_str())
			.map_err(|_| anyhow!("Failed to parse file name to download"))?;
		
		Ok(ModDownloadVersion {
			title: spt_mod.title,
			download_url,
			version: mod_version.version,
			uploaded_at: mod_version.uploaded_at,
			file_name,
		})
	}

	pub async fn get_version(
		&self,
		spt_link: SptLink,
		version: Versioning,
	) -> Result<Option<ModDownloadVersion>> {
		let spt_mod = self.get_all_versions(spt_link).await?;
		let mod_version = spt_mod
			.versions
			.into_iter()
			.find(|mv| mv.version == version);

		let Some(mod_version) = mod_version else {
			return Ok(None);
		};

		let file_name = get_mod_filename(mod_version.download_url.as_str())
			.map_err(|_| anyhow!("Failed to parse file name to download"))?;

		Ok(Some(ModDownloadVersion {
			title: spt_mod.title,
			version: mod_version.version,
			uploaded_at: mod_version.uploaded_at,
			download_url: mod_version.download_url,
			file_name,
		}))
	}

	async fn get_all_versions(&self, spt_link: SptLink) -> Result<SptMod> {
		let url = spt_link.get_versions_page()?;
		let response = self
			.client
			.get(url.clone())
			.send()
			.await?
			.error_for_status()?;
		let document = response.text().await?;
		let mod_versions = html_parsers::spt_parse_mod_page(&document)?;
		Ok(mod_versions)
	}

	async fn get_mod_dl_link(&self, external_url: Url) -> Result<Url> {
		let html = self
			.client
			.get(external_url)
			.send()
			.await?
			.error_for_status()?
			.text()
			.await?;
		html_parsers::spt_parse_download(&html)
	}
}

#[derive(Debug, Clone)]
pub struct SptLink {
	url: Url,
}

impl SptLink {
	pub fn parse(url: &str) -> Result<Self> {
		validate_url(url).map_err(|err| anyhow!("Failed to parse SP Tarkov url: {}", err))?;
		let url = Url::parse(url)?;
		Ok(Self { url })
	}

	fn get_versions_page(&self) -> Result<Url> {
		let url = self.url.join("#versions")?;
		Ok(url)
	}
}

fn validate_url(input: &str) -> PResult<()> {
	let (parsed, _) = "https://hub.sp-tarkov.com/files/file/".parse_peek(input)?;
	let (parsed, numbers) = take_until(1.., "-").parse_peek(parsed)?;
	digit1.and_then(take(numbers.len())).parse_peek(numbers)?;
	let (_parsed, _test) = take_until(0.., "/").parse_peek(parsed)?;
	"/".parse_peek(_parsed)?;
	Ok(())
}

fn get_mod_filename(input: &str) -> PResult<String> {
	let (parsed, _) = "https://".parse_peek(input)?;
	let (file_name, _): (&str, Vec<_>) = repeat(1.., filename_parser).parse_peek(parsed)?;
	Ok(file_name.to_string())
}

fn filename_parser<'a>(input: &mut &'a str) -> PResult<&'a str> {
	take_until(1.., "/").parse_next(input)?;
	take(1usize).parse_next(input)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	#[ignore]
	async fn it_works() {
		let client = SptClient::new(Client::new());
		let spt_mod =
			SptLink::parse("https://hub.sp-tarkov.com/files/file/1963-better-keys-updated/")
				.unwrap();
		let result = client.get_all_versions(spt_mod).await.unwrap();
		assert!(!result.versions.is_empty());
	}

	#[test]
	fn test_filename_parser() {
		let result = get_mod_filename("https://github.com/maxloo2/betterkeys-updated/releases/download/v1.2.3/maxloo2-betterkeys-updated-v1.2.3.zip").unwrap();
		assert_eq!(result, "maxloo2-betterkeys-updated-v1.2.3.zip".to_string());
	}
}
