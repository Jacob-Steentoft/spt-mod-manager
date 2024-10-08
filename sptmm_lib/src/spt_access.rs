mod zip_data;

use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use crate::shared_traits::{ModName, TimeProvider};
use crate::spt_access::zip_data::ZipData;
use anyhow::{anyhow, Context, Result};
use compress_tools::{ArchiveContents, ArchiveIterator, ArchiveIteratorBuilder, Ownership};
use tokio::fs;
use std::fs::File;
use walkdir::WalkDir;
use winnow::combinator::{empty, opt, separated};
use winnow::prelude::*;
use winnow::token::take_until;
use winnow::{dispatch, PResult};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};
use crate::path_access::PathAccess;

const OLD_SERVER_FILE_NAME: &str = "Aki.Server.exe";
const SERVER_FILE_NAME: &str = "SPT.Server.exe";
const BEPINEX_CONFIG_PATH: &str = "BepInEx/config";
const BEPINEX_CACHE_PATH: &str = "BepInEx/cache";
const USER_CACHE_PATH: &str = "user/cache";

#[derive(Clone)]
enum FileType {
	Unknown,
	Client,
	Server,
}

#[derive(Clone, Copy)]
pub enum InstallTarget {
	Server,
	Client,
}

#[derive(Debug, Clone)]
pub struct SptAccess<Time: TimeProvider> {
	server_mods_path: PathBuf,
	client_mods_path: PathBuf,
	root_path: PathBuf,
	time: Time,
	install_index: PathBuf,
}

impl<Time: TimeProvider> SptAccess<Time> {
	pub async fn init(paths: &PathAccess, time: Time) -> Result<Self> {
		let root_path = paths.spt_root();
		if !Path::new(&root_path.join(SERVER_FILE_NAME)).exists() && !Path::new(&root_path.join(OLD_SERVER_FILE_NAME)).exists() {
			return Err(anyhow!("Could not find {SERVER_FILE_NAME} or {OLD_SERVER_FILE_NAME} in the current folder"));
		}
		let install_index = root_path.join("install_hash");
		if !install_index.is_dir() {
			fs::create_dir(&install_index).await?;
		}
		Ok(Self {
			server_mods_path: root_path.join("user/mods/"),
			client_mods_path: root_path.join("BepInEx/plugins/"),
			root_path: PathBuf::from(root_path),
			time,
			install_index,
		})
	}
	pub fn install_mod<P: AsRef<Path>, Mod: ModName>(
		&self,
		mod_archive_path: P,
		spt_mod: &Mod,
		install_target: InstallTarget,
	) -> Result<()> {
		let mut map = HashMap::new();
		let archive_iter = new_file_archive_iter(BufReader::new(File::open(mod_archive_path)?))?;

		let mut buffer = Vec::default();
		let mut zip_path = String::default();
		let mut installed_file_counter = 0;
		for content in archive_iter {
			match content {
				ArchiveContents::StartOfEntry(name, _) => {
					zip_path = name;
				}
				ArchiveContents::DataChunk(mut data) => buffer.append(&mut data),
				ArchiveContents::EndOfEntry => {
					let zip_data = ZipData::new(&buffer, &zip_path);
					if !zip_data.should_install(&install_target) {
						continue;
					}
					map.insert(
						zip_data.get_path().to_string(),
						zip_data.get_hash().to_string(),
					);
					self.write_file_to_tarkov(zip_data)?;
					installed_file_counter += 1;
					buffer = Vec::default();
					zip_path = String::default();
				}
				ArchiveContents::Err(err) => {
					return Err(err.into());
				}
			}
		}

		if installed_file_counter == 0 {
			return Err(anyhow!("No files with a structured installation path was found"));
		}

		let mod_name = self.install_index.join(spt_mod.to_file_name());
		let writer = BufWriter::new(File::create(mod_name)?);
		serde_json::to_writer(writer, &map)?;

		Ok(())
	}

	pub fn is_same_installed_version<P: AsRef<Path>, Mod: ModName>(
		&self,
		mod_archive_path: P,
		mod_name: &Mod,
		install_target: InstallTarget,
	) -> Result<bool> {
		let mod_name = self.install_index.join(mod_name.to_file_name());
		if !mod_name.is_file() {
			return Ok(false);
		}
		let map: HashMap<String, String> =
			serde_json::from_reader(BufReader::new(File::open(mod_name)?))?;
		
		let archive_iter = new_file_archive_iter(BufReader::new(File::open(mod_archive_path)?))?;

		let mut buffer = Vec::default();
		let mut zip_path = String::default();
		for content in archive_iter {
			match content {
				ArchiveContents::StartOfEntry(name, _) => {
					zip_path = name;
				}
				ArchiveContents::DataChunk(mut data) => buffer.append(&mut data),
				ArchiveContents::EndOfEntry => {
					let zip_data = ZipData::new(&buffer, &zip_path);
					if !zip_data.should_install(&install_target) {
						continue;
					}
					if !map
						.get(zip_data.get_path())
						.is_some_and(|str| str == zip_data.get_hash())
					{
						return Ok(false);
					}
					buffer = Vec::default();
					zip_path = String::default();
				}
				ArchiveContents::Err(err) => {
					return Err(err.into());
				}
			}
		}
		Ok(true)
	}

	pub fn install_mod_to_path(
		&self,
		mod_archive_path: impl AsRef<Path>,
		install_path: impl AsRef<Path>,
	) -> Result<()> {
		let reader = BufReader::new(File::open(mod_archive_path)?);
		compress_tools::uncompress_archive(reader, install_path.as_ref(), Ownership::Ignore)?;
		Ok(())
	}
	
	pub async fn clear_mm_cache(&self) -> Result<Vec<OsString>>{
		let mut vec = Vec::new();
		let mut entries = fs::read_dir(&self.install_index).await?;
		while let Some(entry) = entries.next_entry().await? {
			let path = entry.path();
			fs::remove_file(&path).await?;
			vec.push(path.into_os_string());
		}
		Ok(vec)
	}

	pub async fn clear_spt_cache(&self) -> Result<Vec<OsString>>{
		let mut vec = Vec::new();
		let bepinex_path = &self.root_path.join(BEPINEX_CACHE_PATH);
		vec.append(&mut remove_all_files_in_dir(bepinex_path).await?);
		let user_path = &self.root_path.join(USER_CACHE_PATH);
		vec.append(&mut remove_all_files_in_dir(user_path).await?);
		Ok(vec)
	}

	pub async fn clear_spt_config(&self) -> Result<Vec<OsString>>{
		let path = &self.root_path.join(BEPINEX_CONFIG_PATH);
		remove_all_files_in_dir(path).await
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
	
	pub async fn remove_all_mods(&self) -> Result<Vec<OsString>>{
		let mut vec = Vec::new();
		let mut entries = fs::read_dir(&self.server_mods_path).await?;
		while let Some(entry) = entries.next_entry().await? {
			let path = entry.path();
			if path.is_file() {
				continue
			}
			fs::remove_dir_all(&path).await?;
			vec.push(path.into_os_string());
		}
		let mut entries = fs::read_dir(&self.client_mods_path).await?;
		while let Some(entry) = entries.next_entry().await? {
			let path = entry.path();
			if path.file_name() == Some(OsStr::new("spt")) {
				continue
			}
			if path.is_file() {
				fs::remove_file(&path).await?;
				vec.push(path.into_os_string());
				continue
			}
			
			fs::remove_dir_all(&path).await?;
			vec.push(path.into_os_string());
		}
		vec.append(&mut self.clear_mm_cache().await?);
		vec.append(&mut self.clear_spt_cache().await?);
		vec.append(&mut self.clear_spt_config().await?);
		Ok(vec)
	}

	fn write_file_to_tarkov(&self, zip_data: ZipData) -> Result<()> {
		let path = self.root_path.join(zip_data.get_path());
		if let Some(dir_path) = dir_parser(path.to_str().context("Failed to parse install path")?)
			.map_err(|_| anyhow!("Failed to parse install path"))?
		{
			std::fs::create_dir_all(dir_path)?;
		}

		let mut writer = BufWriter::new(File::create(path)?);
		writer.write_all(zip_data.get_data())?;
		Ok(())
	}
}

async fn remove_all_files_in_dir(path: impl AsRef<Path>) -> Result<Vec<OsString>> {
	let path = path.as_ref();
	let mut vec = Vec::new();
	if !path.is_dir() {
		return Ok(vec)
	}
	let mut entries = fs::read_dir(path).await?;
	while let Some(entry) = entries.next_entry().await? {
		let path = entry.path();
		if !path.is_file() {
			continue
		}
		fs::remove_file(&path).await?;
		vec.push(path.into_os_string());
	}
	Ok(vec)
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
fn new_file_archive_iter(reader: BufReader<File>) -> Result<ArchiveIterator<BufReader<File>>> {
	Ok(ArchiveIteratorBuilder::new(reader)
		.filter(|name, _| !name.ends_with('/'))
		.build()?)
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

fn file_parser(file_name: &mut &str) -> FileType {
	let result: PResult<FileType> = dispatch! { take_until(0.., "/");
		"user" => empty.value(FileType::Server),
		"BepInEx" => empty.value(FileType::Client),
		_ => empty.value(FileType::Unknown),
	}
	.parse_next(file_name);
	result.unwrap_or(FileType::Unknown)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::shared_traits::MockTimeProvider;
	use chrono::{DateTime, Utc};

	struct TestModName(String);

	impl ModName for TestModName {
		fn get_name(&self) -> &str {
			&self.0
		}

		fn is_same_name<Name: ModName>(&self, mod_name: &Name) -> bool {
			self.0 == mod_name.get_name()
		}
	}

	#[tokio::test]
	async fn integration_test_restore() {
		let provider = MockTimeProvider::new();
		let buf = PathBuf::from("test_data/backup_2024-06-11T19-06-1718132955Z.zip");
		let path = "./test_output/restore_test";
		fs::create_dir_all(path).await.unwrap();
		let project = PathAccess::from(path, path).unwrap();
		SptAccess::init(&project, provider).await
			.unwrap()
			.restore_from(buf)
			.unwrap();

		assert!(Path::new(&format!(
			"{path}/user/mods/maxloo2-betterkeys-updated/package.json"
		))
		.is_file());
		fs::remove_dir_all(path).await.unwrap()
	}

	#[tokio::test]
	async fn integration_test_install() {
		let provider = MockTimeProvider::new();
		let buf = PathBuf::from("test_data/1.2.3_maxloo2-betterkeys-updated-v1.2.3.zip");
		let path = "./test_output/install_test";
		fs::create_dir_all(path).await.unwrap();
		let project = PathAccess::from(path, path).unwrap();
		SptAccess::init(&project, provider).await
			.unwrap()
			.install_mod(buf, &TestModName("Test".to_string()), InstallTarget::Client)
			.unwrap();
		fs::remove_dir_all(path).await.unwrap()
	}

	#[tokio::test]
	async fn integration_test_backup() {
		let mut provider = MockTimeProvider::new();
		provider
			.expect_get_current_time()
			.returning(DateTime::<Utc>::default);
		let path = PathBuf::from("./test_output/backup_test");
		let _discard = fs::remove_dir_all(&path);
		fs::create_dir_all(&path).await.unwrap();
		let path1 = "./test_data/backed_up_data";
		let project = PathAccess::from(path1, path1).unwrap();

		SptAccess::init(&project, provider).await
			.unwrap()
			.backup_to(&path)
			.unwrap();
		fs::remove_dir_all(&path).await.unwrap()
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
