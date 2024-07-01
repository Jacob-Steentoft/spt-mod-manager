use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use reqwest::Url;
use scraper::selector::CssLocalName;
use scraper::{CaseSensitivity, Element, Html, Selector};
use versions::Versioning;
use winnow::prelude::*;
use winnow::stream::AsChar;
use winnow::token::take_till;
use winnow::PResult;

pub(super) struct SptMod {
	pub title: String,
	pub versions: Vec<SptModVersion>,
}

#[derive(Debug)]
pub(super) struct SptModVersion {
	pub version: Versioning,
	pub download_url: Url,
	pub uploaded_at: DateTime<Utc>,
}

static TIME_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("time").unwrap());
static LINK_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("a").unwrap());
static DIV_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("div").unwrap());
static H1_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("h1").unwrap());
static DOWNLOAD_ELEMENTS: Lazy<Selector> = Lazy::new(|| {
	Selector::parse(r#"li[data-is-deleted="false"][data-is-disabled="false"]"#).unwrap()
});
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
	for element in versions.select(&DOWNLOAD_ELEMENTS) {
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
			.context("Failed to find time for version")?;

		let unix_timestamp = time_value.parse::<i64>()?;
		let time = DateTime::<Utc>::from_timestamp(unix_timestamp, 0)
			.context("Failed to parse the time")?;

		let external_download_link = link
			.attr("href")
			.context("Found no download link for version")?;

		let version_str = link.text().next().context("Found no version name")?;

		let version = parse_version(version_str)
			.ok()
			.flatten()
			.context("Failed to parse version")?;
		download_links.push(SptModVersion {
			version,
			download_url: Url::parse(external_download_link)?,
			uploaded_at: time,
		})
	}
	Ok(SptMod {
		title: title.to_string(),
		versions: download_links,
	})
}

static GOOGLE_DOWNLOAD_FORM: Lazy<Selector> = Lazy::new(|| {
	Selector::parse(r#"form[action="https://drive.usercontent.google.com/download"]"#).unwrap()
});
static HIDDEN_INPUT: Lazy<Selector> =
	Lazy::new(|| Selector::parse(r#"input[type="hidden"]"#).unwrap());
static GOOGLE_WARNING: Lazy<Selector> =
	Lazy::new(|| Selector::parse(r#"p[class="uc-warning-subcaption"]"#).unwrap());
pub fn google_parse_download(document: &str) -> Result<(Url, String)> {
	let html = Html::parse_document(document);
	let download_form = html
		.select(&GOOGLE_DOWNLOAD_FORM)
		.next()
		.context("Failed to find download form")?;
	let file_name = html
		.select(&GOOGLE_WARNING)
		.next()
		.and_then(|e| e.select(&LINK_SELECTOR).next())
		.and_then(|e| e.text().next())
		.context("Failed to find file name")?;
	let download_link = download_form
		.attr("action")
		.context("Failed to find download link")?;
	let mut vec = Vec::new();
	for element in download_form.child_elements() {
		if HIDDEN_INPUT.matches_with_scope(&element, None) {
			let name = element
				.attr("name")
				.context("Failed to parse file download")?;
			let value = element
				.attr("value")
				.context("Failed to parse file download")?;
			vec.push((name, value))
		}
	}
	let download_link = Url::parse_with_params(download_link, &vec)?;
	Ok((download_link, file_name.to_string()))
}

pub fn parse_version(version: &str) -> PResult<Option<Versioning>> {
	let (remainder, _) = take_till(0.., AsChar::is_dec_digit).parse_peek(version)?;
	let version = Versioning::parse(remainder)
		.ok()
		.map(|(_, version)| version);
	Ok(version)
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
	use std::fs::File;
	use std::io::Read;

	use super::*;

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
		println!("{:?}", vec.versions);
		for element in &vec.versions {
			assert!(element.version.is_ideal())
		}
		assert_eq!(vec.versions.len(), 7);
	}
}
