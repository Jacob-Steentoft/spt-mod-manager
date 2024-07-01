use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use tokio::time::{Instant, sleep_until};
use url::Url;
use versions::Versioning;
use winnow::ascii::digit1;
use winnow::combinator::{eof, opt, repeat};
use winnow::prelude::*;
use winnow::token::{take, take_until};

use crate::remote_mod_access::{html_parsers, ModDownloadVersion};
use crate::remote_mod_access::html_parsers::SptMod;

pub struct SptClient {
	client: Client,
	last_request: Instant,
	request_delay: Duration,
}

#[derive(Clone)]
enum DownloadLink{
	File{
		file_name: String,
	},
	GoogleDrive{file_id: String},
	Unknown,
}

impl SptClient {
	pub fn new(client: Client) -> Self {
		Self { client, last_request: Instant::now(), request_delay: Duration::from_millis(500) }
	}

	pub async fn get_latest_version(&mut self, spt_link: SptLink) -> Result<ModDownloadVersion> {
		let spt_mod = self.get_all_versions(spt_link).await?;
		let mod_version = spt_mod
			.versions
			.into_iter()
			.max_by(|x, x1| x.version.cmp(&x1.version))
			.context("Found no mods")?;

		let (download_url, file_name) = self.parse_download(mod_version.download_url).await?;
		
		Ok(ModDownloadVersion {
			title: spt_mod.title,
			download_url,
			version: mod_version.version,
			uploaded_at: mod_version.uploaded_at,
			file_name,
		})
	}

	pub async fn get_version(
		&mut self,
		spt_link: SptLink,
		version: &Versioning,
	) -> Result<Option<ModDownloadVersion>> {
		let spt_mod = self.get_all_versions(spt_link).await?;
		let mod_version = spt_mod
			.versions
			.into_iter()
			.find(|mv| &mv.version == version);

		let Some(mod_version) = mod_version else {
			return Ok(None);
		};

		let (download_url, file_name) = self.parse_download(mod_version.download_url).await?;

		Ok(Some(ModDownloadVersion {
			title: spt_mod.title,
			version: mod_version.version,
			uploaded_at: mod_version.uploaded_at,
			download_url,
			file_name,
		}))
	}

	async fn parse_download(&mut self, url: Url) -> Result<(Url, String)> {
		let download_url = self.get_mod_dl_link(url).await?;

		let (download_url, file_name) = match parse_download_link(&download_url) {
			DownloadLink::File { file_name } => (download_url, file_name),
			DownloadLink::GoogleDrive { file_id } => {
				let url = Url::parse(&format!("https://drive.usercontent.google.com/download?id={file_id}"))?;
				let html = self.get_html(&url).await?;
				html_parsers::google_parse_download(&html)?
			}
			DownloadLink::Unknown => {
				let error = anyhow!("Failed to parse file to download for url: {}", download_url);
				return Err(error)
			}
		};
		Ok((download_url, file_name))
	}

	async fn get_all_versions(&mut self, spt_link: SptLink) -> Result<SptMod> {
		let url = spt_link.get_versions_page()?;
		let html = self.get_spt_html(&url).await?;
		let mod_versions = html_parsers::spt_parse_mod_page(&html).map_err(|err| anyhow!(err))?;
		Ok(mod_versions)
	}

	async fn get_mod_dl_link(&mut self, external_url: Url) -> Result<Url> {
		let html = self.get_spt_html(&external_url).await?;
		html_parsers::spt_parse_download(&html)
	}

	async fn get_spt_html(&mut self, url: &Url) -> Result<String>{
		sleep_until( self.last_request + self.request_delay).await;
		self.last_request = Instant::now();
		let html = self
			.client
			.get(url.clone())
			.send()
			.await?
			.error_for_status()?
			.text()
			.await?;
		Ok(html)
	}
	async fn get_html(&self, url: &Url) -> Result<String>{
		let html = self
			.client
			.get(url.clone())
			.send()
			.await?
			.error_for_status()?
			.text()
			.await?;
		Ok(html)
	}
}

#[derive(Debug, Clone)]
pub struct SptLink {
	link: Url,
}

impl SptLink {
	pub fn parse<S: AsRef<str>>(url: S) -> Result<Self> {
		let url = url.as_ref();
		// TODO: Improve validation to return file name
		validate_url(url).map_err(|_| anyhow!("Failed to parse SP Tarkov url"))?;
		let link = if !url.ends_with('/') {
			Url::parse(&format!("{}/", url))?
		}
		else {
			Url::parse(url)?
		};
		Ok(Self { link })
	}

	fn get_versions_page(&self) -> Result<Url> {
		let url = self.link.join("#versions")?;
		Ok(url)
	}
	
	pub fn starts_with_host<S: AsRef<str>>(url: S) -> bool{
		url.as_ref().starts_with("https://hub.sp-tarkov.com")
	}
}

fn validate_url(input: &str) -> PResult<()> {
	let (remainder, _) = "https://hub.sp-tarkov.com/files/file/".parse_peek(input)?;
	let (remainder, numbers) = take_until(1.., "-").parse_peek(remainder)?;
	digit1.and_then(take(numbers.len())).parse_peek(numbers)?;
	let (remainder, taken) = opt((take_until(1.., "/"), take(1usize))).parse_peek(remainder)?;
	if taken.is_some() {
		eof.parse_peek(remainder)?;
	}
	Ok(())
}

fn parse_download_link(download_link: &Url) -> DownloadLink{
	let str = download_link.as_str();
	if let Ok(file_name) = get_mod_filename(str) {
		return DownloadLink::File{file_name }
	}
	if let Ok(file_id) = get_google_file_id(str) {
		return DownloadLink::GoogleDrive{file_id}
	}

	DownloadLink::Unknown
}

fn get_google_file_id(input: &str) -> PResult<String> {
	let (parsed, _) = "https://drive.google.com/file/d/".parse_peek(input)?;
	let (_, file_id) = take_until(1.., "/").parse_peek(parsed)?;

	Ok(file_id.to_string())
}

fn get_mod_filename(input: &str) -> PResult<String> {
	let (parsed, _) = "https://".parse_peek(input)?;
	let (file_name, _): (&str, Vec<_>) = repeat(1.., filename_parser).parse_peek(parsed)?;
	take_until(1.., '.').parse_peek(file_name)?;
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
		let mut client = SptClient::new(Client::new());
		let spt_mod =
			SptLink::parse("https://hub.sp-tarkov.com/files/file/1963-better-keys-updated/")
				.unwrap();
		let result = client.get_all_versions(spt_mod).await.unwrap();
		assert!(!result.versions.is_empty());
	}

	#[test]
	fn url_parses_correctly_with_slash() {
		let result = validate_url("https://hub.sp-tarkov.com/files/file/1963-better-keys-updated/");
		assert!(result.is_ok());
	}

	#[test]
	fn url_parses_correctly_without_slash() {
		let result = validate_url("https://hub.sp-tarkov.com/files/file/1963-better-keys-updated");
		assert!(result.is_ok());
	}

	#[test]
	fn url_parses_incorrectly_with_ext() {
		let result = validate_url("https://hub.sp-tarkov.com/files/file/1963-better-keys-updated/#versions");
		assert!(result.is_err());
	}

	#[test]
	fn test_filename_parser() {
		let result = get_mod_filename("https://github.com/maxloo2/betterkeys-updated/releases/download/v1.2.3/maxloo2-betterkeys-updated-v1.2.3.zip").unwrap();
		assert_eq!(result, "maxloo2-betterkeys-updated-v1.2.3.zip".to_string());
	}
}
