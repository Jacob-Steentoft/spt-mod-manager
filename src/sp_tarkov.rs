use crate::mod_manager::ModVersion;
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use reqwest::{Client, ClientBuilder, Url};
use scraper::selector::CssLocalName;
use scraper::{CaseSensitivity, Element, Html, Selector};
use std::fs::File;
use winnow::ascii::digit1;
use winnow::prelude::*;
use winnow::token::{take, take_until};

pub struct SptClient {
	client: Client,
}

impl SptClient {
	pub fn new() -> Self {
		let client = ClientBuilder::new()
			.user_agent("spt_mod_manager_rs")
			.build()
			.unwrap();
		Self { client }
	}

	pub async fn get_latest_version(&self, spt_link: SptLink) -> Result<SptModVersion> {
		let download_links = self.get_all_versions(spt_link).await?;
		let mod_version = download_links
			.into_iter()
			.min_by(|x, x1| x.time.cmp(&x1.time))
			.context("Found no mods")?;

		Ok(mod_version)
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
		let download_links = parse_document_for_versions(&document)?;
		Ok(download_links)
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
	download_url: Url,
	time: DateTime<Utc>,
}

impl ModVersion for SptModVersion {
	fn get_version(&self) -> String {
		self.version.clone()
	}

	async fn download(&self, download_dir: File) -> Result<()> {
		let download_page_html = reqwest::get(self.download_url.clone())
			.await?
			.error_for_status()?
			.text()
			.await?;

		Ok(())
	}
}

async fn get_mod_versions(spt_mod: SptLink) -> Result<Vec<SptModVersion>> {
	let url = spt_mod.get_versions_page()?;
	let response = reqwest::get(url.clone()).await?.error_for_status()?;
	let document = response.text().await?;
	let download_links = parse_document_for_versions(&document)?;

	Ok(download_links)
}

fn parse_document_for_versions(document: &str) -> Result<Vec<SptModVersion>> {
	let html = Html::parse_document(document);
	let versions_selector = Selector::parse("div").unwrap();
	let versions = html
		.select(&versions_selector)
		.find(|e| {
			e.has_id(
				&CssLocalName::from("versions"),
				CaseSensitivity::CaseSensitive,
			)
		})
		.context("Found no versions")?;

	let list_selector = Selector::parse("li").unwrap();
	let link_selector = Selector::parse("a").unwrap();
	let time_selector = Selector::parse("time").unwrap();
	let mut download_links = Vec::new();
	for element in versions.select(&list_selector) {
		let Some(link) = element.select(&link_selector).find(|e| {
			e.has_class(
				&CssLocalName::from("externalURL"),
				CaseSensitivity::CaseSensitive,
			)
		}) else {
			continue;
		};
		let time_value = element
			.select(&time_selector)
			.next()
			.and_then(|e| e.attr("data-timestamp"))
			.context("Failed to find time")?;

		let unix_timestamp = time_value.parse::<i64>()?;
		let time = DateTime::<Utc>::from_timestamp(unix_timestamp, 0)
			.context("Failed to parse the time")?;

		let external_download_link = link
			.attr("href")
			.context("Found no download link for version")?;

		let version = link
			.text()
			.next()
			.context("Found no version name")?
			.to_string();

		download_links.push(SptModVersion {
			version,
			download_url: Url::parse(external_download_link)?,
			time,
		})
	}
	Ok(download_links)
}

fn validate_url(input: &str) -> PResult<()> {
	let (parsed, _) = "https://hub.sp-tarkov.com/files/file/".parse_peek(input)?;
	let (parsed, numbers) = take_until(1.., "-").parse_peek(parsed)?;
	digit1.and_then(take(numbers.len())).parse_peek(numbers)?;
	let (_parsed, _test) = take_until(0.., "/").parse_peek(parsed)?;
	"/".parse_peek(_parsed)?;
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::io::Read;

	#[tokio::test]
	#[ignore]
	async fn it_works() {
		let spt_mod =
			SptLink::parse("https://hub.sp-tarkov.com/files/file/1963-better-keys-updated/")
				.unwrap();
		let result = get_mod_versions(spt_mod).await.unwrap();
		assert!(!result.is_empty());
	}

	#[test]
	fn test_version_parser() {
		let mut buffer = String::new();
		File::open("test_data/spt_versions.html")
			.unwrap()
			.read_to_string(&mut buffer)
			.unwrap();
		let vec = parse_document_for_versions(&buffer).unwrap();
		for element in &vec {
			assert!(element.version.starts_with("Version "))
		}
		assert_eq!(vec.len(), 7);
	}
}
