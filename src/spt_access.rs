use std::fs;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use crate::shared_traits::TimeProvider;
use anyhow::{anyhow, Result};
use walkdir::WalkDir;
use winnow::combinator::{empty, opt, separated};
use winnow::prelude::*;
use winnow::token::take_until;
use winnow::{dispatch, PResult};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

pub struct SptAccess<Time: TimeProvider> {
	server_mods_path: PathBuf,
	client_mods_path: PathBuf,
	root_path: PathBuf,
	time: Time,
}

#[derive(Clone)]
enum FileType {
	Unknown,
	Client,
	Server,
}

pub enum InstallTarget {
	Server,
	Client,
}

impl<Time: TimeProvider> SptAccess<Time> {
	pub fn new<P: AsRef<Path>>(root_path: P, time: Time) -> Self {
		Self {
			server_mods_path: root_path.as_ref().join("user/mods/"),
			client_mods_path: root_path.as_ref().join("BepInEx/plugins/"),
			root_path: PathBuf::from(root_path.as_ref()),
			time,
		}
	}
	pub fn install_mod<P: AsRef<Path>>(
		&self,
		mod_archive_path: P,
		target: InstallTarget,
	) -> Result<()> {
		let reader = BufReader::new(File::open(mod_archive_path)?);
		let mut zip = ZipArchive::new(reader)?;
		let names: Vec<_> = zip.file_names().map(|str| str.to_string()).collect();
		for name in names {
			let file_type =
				file_parser(&mut name.as_str()).map_err(|_| anyhow!("Failed to parse folder"))?;
			match (&file_type, &target) {
				(FileType::Client, InstallTarget::Client) => {
					install_if_file(&mut zip, &name)?;
				}
				(FileType::Server, _) => {
					install_if_file(&mut zip, &name)?;
				}
				_ => {}
			};
		}
		Ok(())
	}
	
	pub fn install_mod_to_path<P: AsRef<Path>>(&self, mod_archive_path: P, install_path: P) -> Result<()>{
		let reader = BufReader::new(File::open(mod_archive_path)?);
		let mut archive = ZipArchive::new(reader)?;
		archive.extract(install_path)?;
		Ok(())
	}

	pub fn backup_to<P: AsRef<Path>>(&self, archive_path: P) -> Result<()> {
		let current_date = self.time.get_current_time();
		let backup_name = format!("backup_{}.zip", current_date.format("%Y-%m-%dT%H-%m-%SZ"));
		let zip_path = archive_path.as_ref().join(backup_name);
		let writer = BufWriter::new(File::create_new(zip_path)?);
		let mut zip_writer = ZipWriter::new(writer);

		backup_folder_content(&mut zip_writer, &self.server_mods_path)?;
		backup_folder_content(&mut zip_writer, &self.client_mods_path)?;
		zip_writer.finish()?;
		Ok(())
	}

	pub fn restore_from<P: AsRef<Path>>(&self, archive_path: P) -> Result<()> {
		let mut zip_archive = ZipArchive::new(File::open(archive_path)?)?;

		zip_archive.extract(&self.root_path)?;
		Ok(())
	}
}

fn install_if_file(zip: &mut ZipArchive<BufReader<File>>, name: &String) -> Result<()> {
	let mut zip_file = zip.by_name(&name)?;
	if !zip_file.is_file() {
		return Ok(());
	}
	let path = format!("./{name}");
	if let Some(dir_path) =
		dir_parser(&path).map_err(|_| anyhow!("Failed to parse install path"))?
	{
		fs::create_dir_all(dir_path)?;
	}

	let mut buffer = Vec::new();
	let mut writer = BufWriter::new(File::create(path)?);
	zip_file.read_to_end(&mut buffer)?;
	writer.write_all(&buffer)?;
	Ok(())
}

fn backup_folder_content(
	zip_writer: &mut ZipWriter<BufWriter<File>>,
	path_buf: &PathBuf,
) -> Result<()> {
	if !path_buf.is_dir() {
		return Ok(());
	}

	let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
	let filter = WalkDir::new(path_buf)
		.into_iter()
		.filter(|x| x.as_ref().is_ok_and(|e| e.path().is_file()));
	for file_entry in filter {
		let file_entry = file_entry?;
		let file_path = file_entry.path();
		let mut buffer = Vec::new();
		let mut file = File::open(file_path)?;
		file.read_to_end(&mut buffer)?;
		zip_writer.start_file_from_path(file_path, options)?;
		zip_writer.write_all(&buffer)?;
	}

	Ok(())
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

fn file_parser(file_name: &mut &str) -> PResult<FileType> {
	(dispatch! { take_until(1.., "/");
		"user" => empty.value(FileType::Server),
		"BepInEx" => empty.value(FileType::Client),
		_ => empty.value(FileType::Unknown),
	})
	.parse_next(file_name)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::shared_traits::MockTimeProvider;
	use chrono::{DateTime, Utc};

	#[test]
	fn integration_test_restore() {
		let provider = MockTimeProvider::new();
		let _result = fs::remove_dir_all("./user");
		let buf = PathBuf::from("test_data/backup_2024-06-11T19-06-1718132955Z.zip");
		let path = "./test_output/restore";
		fs::create_dir_all(path).unwrap();
		SptAccess::new(path, provider).restore_from(buf).unwrap();

		assert!(Path::new(&format!(
			"{path}/user/mods/maxloo2-betterkeys-updated/package.json"
		))
		.is_file());
		fs::remove_dir_all(path).unwrap()
	}

	#[test]
	fn integration_test_install() {
		let provider = MockTimeProvider::new();
		let buf = PathBuf::from("test_data/1.2.3_maxloo2-betterkeys-updated-v1.2.3.zip");
		SptAccess::new("./", provider).install_mod(buf, InstallTarget::Client).unwrap();
	}

	#[test]
	fn integration_test_backup() {
		let mut provider = MockTimeProvider::new();
		provider
			.expect_get_current_time()
			.returning(DateTime::<Utc>::default);
		let path = PathBuf::from("./test_output/backup");
		fs::create_dir_all(&path).unwrap();
		SptAccess::new("./", provider).backup_to(&path).unwrap();
		fs::remove_dir_all(path).unwrap()
	}

	#[test]
	fn when_parsing_multiple_dirs_return_last_dir() {
		let buf = dir_parser("test_data/1.2.3_/maxloo2-betterkeys-updated/-v1.2.3.zip").unwrap();
		assert_eq!(buf, Some("test_data/1.2.3_/maxloo2-betterkeys-updated"))
	}

	#[test]
	fn when_parsing_no_dirs_return_none() {
		let buf = dir_parser("test_data").unwrap();
		assert_eq!(buf, None)
	}
}
