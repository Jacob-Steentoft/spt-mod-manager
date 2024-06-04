use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use reqwest::Url;
use scraper::selector::CssLocalName;
use scraper::{CaseSensitivity, Element, Html, Selector};

pub(super) struct SptMod {
	pub title: String,
	pub versions: Vec<SptModVersion>,
}

pub(super) struct SptModVersion {
	pub version: String,
	pub download_url: Url,
	pub uploaded_at: DateTime<Utc>,
}

static TIME_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("time").unwrap());
static LINK_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("a").unwrap());
static DIV_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("div").unwrap());
static H1_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("h1").unwrap());
static LIST_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("li").unwrap());
static VERSIONS_CSS: Lazy<CssLocalName> = Lazy::new(|| CssLocalName::from("versions"));
static CONTENT_TITLE_CSS: Lazy<CssLocalName> = Lazy::new(|| CssLocalName::from("contentTitle"));
static URL_CSS: Lazy<CssLocalName> = Lazy::new(|| CssLocalName::from("externalURL"));

pub fn spt_parse_mod_page(document: &str) -> Result<SptMod> {
	let html = Html::parse_document(document);
	let title = html
		.select(&H1_SELECTOR)
		.find(|e| e.has_class(&CONTENT_TITLE_CSS, CaseSensitivity::CaseSensitive))
		.and_then(|e| {
			e.child_elements()
				.find(|n| n.attr("itemprop").is_some_and(|x| x == "name"))
		})
		.and_then(|e| e.text().next())
		.context("Found no title")?;

	let versions = html
		.select(&DIV_SELECTOR)
		.find(|e| e.has_id(&VERSIONS_CSS, CaseSensitivity::CaseSensitive))
		.context("Found no versions")?;

	let mut download_links = Vec::new();
	for element in versions.select(&LIST_SELECTOR) {
		let Some(link) = element
			.select(&LINK_SELECTOR)
			.find(|e| e.has_class(&URL_CSS, CaseSensitivity::CaseSensitive))
		else {
			continue;
		};
		let time_value = element
			.select(&TIME_SELECTOR)
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
			uploaded_at: time,
		})
	}
	Ok(SptMod {
		title: title.to_string(),
		versions: download_links
	})
}

pub fn spt_parse_download(document: &str) -> Result<Url> {
	let html = Html::parse_document(document);
	let url_str = html
		.select(&LINK_SELECTOR)
		.next()
		.and_then(|e| e.attr("href"))
		.context("Found no link on the download page")?;
	let url = Url::parse(url_str)?;
	Ok(url)
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::fs::File;
	use std::io::Read;

	#[test]
	fn test_parse_download() {
		let mut buffer = String::new();
		File::open("test_data/spt_external_download.html")
			.unwrap()
			.read_to_string(&mut buffer)
			.unwrap();
		let url = spt_parse_download(&buffer).unwrap();
		assert_eq!(url, Url::parse("https://github.com/maxloo2/betterkeys-updated/releases/download/v1.2.3/maxloo2-betterkeys-updated-v1.2.3.zip").unwrap())
	}

	#[test]
	fn test_version_parser() {
		let mut buffer = String::new();
		File::open("test_data/spt_versions.html")
			.unwrap()
			.read_to_string(&mut buffer)
			.unwrap();
		let vec = spt_parse_mod_page(&buffer).unwrap();
		assert_eq!(vec.title, "Better Keys Updated".to_string());
		for element in &vec.versions {
			assert!(element.version.starts_with("Version "))
		}
		assert_eq!(vec.versions.len(), 7);
	}
}
