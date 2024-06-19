use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use crate::shared_traits::{ModName, TimeProvider};
use anyhow::{anyhow, Context, Result};
use walkdir::WalkDir;
use winnow::combinator::{empty, opt, separated};
use winnow::prelude::*;
use winnow::token::take_until;
use winnow::{dispatch, PResult};
use zip::read::ZipFile;
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

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

struct ZipData {
	data: Vec<u8>,
	hash: String,
	zip_path: String,
	file_type: FileType,
}

impl ZipData {
	fn try_from(zip_file: ZipFile) -> Result<Option<Self>> {
		if !zip_file.is_file() {
			return Ok(None);
		}
		let zip_path = zip_file
			.enclosed_name()
			.and_then(|pb| pb.to_str().map(|str| str.to_string()))
			.context(format!(
				"Failed to get zip file path for: {}",
				zip_file.name()
			))?;
		let (data, hash) = get_data_and_hash(zip_file)?;
		let file_type = file_parser(&mut zip_path.as_str());
		Ok(Some(Self {
			hash,
			data,
			zip_path,
			file_type,
		}))
	}

	fn should_install(&self, target: &InstallTarget) -> bool {
		matches!(
			(&self.file_type, target),
			(FileType::Client, InstallTarget::Client) | (FileType::Server, _)
		)
	}
}

pub struct SptAccess<Time: TimeProvider> {
	server_mods_path: PathBuf,
	client_mods_path: PathBuf,
	root_path: PathBuf,
	time: Time,
	install_index: PathBuf,
}

impl<Time: TimeProvider> SptAccess<Time> {
	pub fn init<P: AsRef<Path>>(root_path: P, time: Time) -> Result<Self> {
		let path = root_path.as_ref();
		let install_index = path.join("install_hash");
		if !install_index.is_dir() {
			fs::create_dir(&install_index)?;
		}
		Ok(Self {
			server_mods_path: path.join("user/mods/"),
			client_mods_path: path.join("BepInEx/plugins/"),
			root_path: PathBuf::from(path),
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
		let mut zip = ZipArchive::new(BufReader::new(File::open(mod_archive_path)?))?;
		let zip_length = zip.len();
		for index in 0..zip_length {
			let Some(zip_data) = ZipData::try_from(zip.by_index(index)?)? else {
				continue;
			};
			if !zip_data.should_install(&install_target) {
				continue;
			}
			map.insert(zip_data.zip_path.clone(), zip_data.hash.clone());
			self.write_file_to_tarkov(zip_data)?;
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
		let map: HashMap<String, String> = serde_json::from_reader(BufReader::new(File::open(mod_name)?))?;
		let mut zip = ZipArchive::new(BufReader::new(File::open(mod_archive_path)?))?;
		let zip_length = zip.len();
		for index in 0..zip_length {
			let Some(zip_data) = ZipData::try_from(zip.by_index(index)?)? else {
				continue;
			};
			if !zip_data.should_install(&install_target) {
				continue;
			}
			if !map
				.get(&zip_data.zip_path)
				.is_some_and(|str| str == &zip_data.hash)
			{
				return Ok(false);
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

	fn write_file_to_tarkov(&self, zip_data: ZipData, ) -> Result<()> {
		let path = self.root_path.join(zip_data.zip_path);
		if let Some(dir_path) = dir_parser(path.to_str().context("Failed to parse install path")?).map_err(|_| anyhow!("Failed to parse install path"))?
		{
			fs::create_dir_all(dir_path)?;
		}

		let mut writer = BufWriter::new(File::create(path)?);
		writer.write_all(&zip_data.data)?;
		Ok(())
	}
}

fn get_data_and_hash(mut zip: ZipFile) -> Result<(Vec<u8>, String)> {
	let mut buffer = Vec::new();
	zip.read_to_end(&mut buffer)?;
	let hash = sha256::digest(&buffer);
	Ok((buffer, hash))
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

	#[test]
	fn integration_test_restore() {
		let provider = MockTimeProvider::new();
		let buf = PathBuf::from("test_data/backup_2024-06-11T19-06-1718132955Z.zip");
		let path = "./test_output/restore_test";
		fs::create_dir_all(path).unwrap();
		SptAccess::init(path, provider)
			.unwrap()
			.restore_from(buf)
			.unwrap();

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
		let path = "./test_output/install_test";
		fs::create_dir_all(path).unwrap();
		SptAccess::init(path, provider)
			.unwrap()
			.install_mod(buf, &TestModName("Test".to_string()), InstallTarget::Client)
			.unwrap();
		fs::remove_dir_all(path).unwrap()
	}

	#[test]
	fn integration_test_backup() {
		let mut provider = MockTimeProvider::new();
		provider
			.expect_get_current_time()
			.returning(DateTime::<Utc>::default);
		let path = PathBuf::from("./test_output/backup_test");
		let _discard = fs::remove_dir_all(&path);
		fs::create_dir_all(&path).unwrap();
		SptAccess::init("./test_data/backed_up_data", provider)
			.unwrap()
			.backup_to(&path)
			.unwrap();
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
