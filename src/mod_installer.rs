use std::fs;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use winnow::{dispatch, PResult};
use winnow::combinator::{empty, opt, separated};
use winnow::prelude::*;
use winnow::token::take_until;
use zip::ZipArchive;

pub struct ModInstaller {}

#[derive(Clone)]
enum FileType {
	Unknown,
	Client,
	Server,
}

impl ModInstaller {
	pub fn new() -> Self {
		Self {}
	}
	pub fn install_for_client(&self, archive_path: PathBuf) -> Result<()> {
		let reader = BufReader::new(File::open(archive_path)?);
		let mut zip = ZipArchive::new(reader)?;
		let names: Vec<_> = zip.file_names().map(|str| str.to_string()).collect();
		for name in names {
			let file_type =
				file_parser(&mut name.as_str()).map_err(|_| anyhow!("Failed to parse folder"))?;
			match file_type {
				FileType::Server | FileType::Client => {
					let mut zip_file = zip.by_name(&name)?;
					if zip_file.is_file() {
						let path = format!("./{name}");
						if let Some(dir_path) = dir_parser(&path)
							.map_err(|_| anyhow!("Failed to parse install path"))?
						{
							fs::create_dir_all(dir_path)?;
						}

						let mut buffer = Vec::new();
						let mut writer = BufWriter::new(File::create(path)?);
						zip_file.read_to_end(&mut buffer)?;
						writer.write_all(&buffer)?;
					}
				}
				FileType::Unknown => {}
			};
		}
		Ok(())
	}
}

fn dir_parser(file_path: &str) -> PResult<Option<&str>> {
	let (_, parsed): (&str, Option<Vec<_>>) =
		opt(separated(1.., take_until(0.., "/"), "/")).parse_peek(file_path)?;
	let Some(parsed) = parsed else {
		return Ok(None);
	};

	let length = parsed
		.iter()
		.fold(0, |counter, data| counter + data.len() + 1);
	Ok(Some(&file_path[..length - 1]))
}

fn file_parser<'a>(file_name: &mut &str) -> PResult<FileType> {
	dispatch! { take_until(1.., "/");
		"user" => empty.value(FileType::Server),
		"BepInEx" => empty.value(FileType::Client),
		_ => empty.value(FileType::Unknown),
	}
	.parse_next(file_name)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_parse_download() {
		let buf = PathBuf::from("test_data/1.2.3_maxloo2-betterkeys-updated-v1.2.3.zip");
		ModInstaller::new().install_for_client(buf).unwrap();
	}

	#[test]
	fn test_parse_multiple_dirs() {
		let buf = dir_parser("test_data/1.2.3_/maxloo2-betterkeys-updated/-v1.2.3.zip").unwrap();
		assert_eq!(buf, Some("test_data/1.2.3_/maxloo2-betterkeys-updated"))
	}

	#[test]
	fn test_parse_no_dirs() {
		let buf = dir_parser("test_data").unwrap();
		assert_eq!(buf, None)
	}
}
