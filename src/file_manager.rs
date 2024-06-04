use crate::mod_downloader::ModVersion;
use anyhow::{anyhow, Result};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::fs::DirEntry;
use std::ops::{Add, AddAssign};
use std::path::PathBuf;

pub struct FileManager {
	cache_dir: PathBuf,
}

enum ModCacheStatus {
	NotCached,
	SameVersion,
	OlderVersion,
}
impl FileManager {
	pub fn new(cache_path: &str) -> Result<Self> {
		let cache_dir = PathBuf::from(cache_path);
		if !cache_dir.is_dir() {
			return Err(anyhow!("The path provided was not a folder"));
		}
		Ok(Self { cache_dir })
	}

	pub fn get_cache_status(&self, mod_version: &ModVersion) -> Result<bool> {
		let folder_name = create_folder_name(mod_version);
		let folder_path = self.cache_dir.join(folder_name);
		if !folder_path.is_dir() {
			fs::create_dir(&folder_path)?;
		}
		let file_prefix = create_mod_file_prefix(mod_version);
		let file_name = create_file_name(mod_version);
		
		// TODO: create normal iterator
		let result = fs::read_dir(folder_path)?.filter_map(|entry| {
			entry.as_ref().and_then(|e| {
				if e.file_name()
					.to_str()
					.is_some_and(|str| str.starts_with(&file_prefix)) {
					Ok(Some(e))
				}
				else {
					Ok(None)
				}
			})
				.ok()
		}).collect::<Vec<_>>();

		!self.current_mods
	}
}

fn create_folder_name(mod_version: &ModVersion) -> OsString {
	mod_version
		.title
		.chars()
		.map(space_mapper)
		.collect::<String>()
		.into()
}

fn create_file_name(mod_version: &ModVersion) -> OsString {
	let string = create_mod_file_prefix(mod_version);
	format!("{string}_{}", mod_version.file_name);
	string.into()
}

fn create_mod_file_prefix(mod_version: &ModVersion) -> String {
	mod_version
		.version
		.chars()
		.map(space_mapper)
		.collect::<String>()
}

fn space_mapper(c: char) -> char {
	match c {
		' ' => '_',
		'-' => '_',
		_ => c,
	}
}
