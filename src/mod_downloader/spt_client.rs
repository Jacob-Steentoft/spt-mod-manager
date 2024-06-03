use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use reqwest::{Client, Url};
use winnow::ascii::digit1;
use winnow::combinator::repeat;
use winnow::prelude::*;
use winnow::token::{take, take_until};

use crate::mod_downloader::{html_parsers, ModVersion};

pub struct SptClient {
	client: Arc<Client>,
}

impl SptClient {
	pub fn new(client: Arc<Client>) -> Self {
		Self { client }
	}

	pub async fn get_latest_version(&self, spt_link: SptLink) -> Result<ModVersion> {
		let download_links = self.get_all_versions(spt_link).await?;
		let mod_version = download_links
			.into_iter()
			.min_by(|x, x1| x.time.cmp(&x1.time))
			.context("Found no mods")?;

		let download_url = self.get_mod_dl_link(mod_version.ext_download_url).await?;

		let file_name = get_mod_filename(download_url.as_str())
			.map_err(|_| anyhow!("Failed to parse file name to download"))?;
		Ok(ModVersion {
			download_url,
			version: mod_version.version,
			uploaded_at: mod_version.time,
			file_name,
		})
	}

	pub async fn get_version(
		&self,
		spt_link: SptLink,
		version: &str,
	) -> Result<Option<SptModVersion>> {
		let download_links = self.get_all_versions(spt_link).await?;
		let mod_version = download_links.into_iter().find(|mv| mv.version == version);

		Ok(mod_version)
	}

	async fn get_all_versions(&self, spt_link: SptLink) -> Result<Vec<SptModVersion>> {
		let url = spt_link.get_versions_page()?;
		let response = self
			.client
			.get(url.clone())
			.send()
			.await?
			.error_for_status()?;
		let document = response.text().await?;
		let download_links = html_parsers::spt_parse_for_versions(&document)?
			.into_iter()
			.map(|mv| SptModVersion {
				ext_download_url: mv.download_url,
				time: mv.time,
				version: mv.version,
			})
			.collect();
		Ok(download_links)
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

#[derive(Debug)]
pub struct SptModVersion {
	version: String,
	ext_download_url: Url,
	time: DateTime<Utc>,
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
		let arc = Arc::new(Client::new());
		let client = SptClient::new(arc);
		let spt_mod =
			SptLink::parse("https://hub.sp-tarkov.com/files/file/1963-better-keys-updated/")
				.unwrap();
		let result = client.get_all_versions(spt_mod).await.unwrap();
		assert!(!result.is_empty());
	}

	#[test]
	fn test_filename_parser() {
		let result = get_mod_filename("https://github.com/maxloo2/betterkeys-updated/releases/download/v1.2.3/maxloo2-betterkeys-updated-v1.2.3.zip").unwrap();
		assert_eq!(result, "maxloo2-betterkeys-updated-v1.2.3.zip".to_string());
	}
}
