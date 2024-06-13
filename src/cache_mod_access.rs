use std::cmp::Ordering;
use std::ffi::OsString;
use std::fs;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;
use tokio::pin;
use winnow::combinator::separated;
use winnow::prelude::*;
use winnow::PResult;
use winnow::token::take_until;

use crate::cache_mod_access::cached_mod::CachedMod;
use crate::cache_mod_access::cached_mod_version::CachedModVersion;
use crate::cache_mod_access::mod_manifest::ModManifest;
use crate::shared_traits::{ModName, ModVersion, ModVersionDownload};

mod cached_mod_version;
mod cached_mod;
mod mod_manifest;

pub struct CacheModAccess {
	cache_dir: PathBuf,
	cached_mods: Vec<CachedMod>,
}

struct CacheFile {
	file_name: String,
	file_ext: Option<String>,
	path: PathBuf,
}

impl CacheFile {
	fn is_manifest(&self) -> bool {
		self.file_ext.as_ref().is_some_and(|s| s == ".manifest")
	}
	fn delete(self) -> Result<()> {
		fs::remove_file(self.path)?;
		Ok(())
	}
}

pub enum ModCacheStatus {
	NotCached,
	NewerVersion,
	SameVersion,
	OlderVersion,
}

impl CacheModAccess {
	pub fn build(cache_path: &str) -> Result<Self> {
		let cache_dir = PathBuf::from(cache_path);
		if !cache_dir.is_dir() {
			return Err(anyhow!("The path provided was not a directory"));
		}
		let cached_mods = calculate_cache(cache_path)?;

		Ok(Self {
			cache_dir,
			cached_mods,
		})
	}

	pub fn get_status<Version: ModVersion>(&self, mod_version: &Version) -> ModCacheStatus {
		let Some(cached_mod) = self
			.cached_mods
			.iter()
			.find(|x| x.is_same_name(mod_version))
		else {
			return ModCacheStatus::NotCached;
		};
		let Some(cached_mod_version) = cached_mod.get_newest() else {
			return ModCacheStatus::NotCached;
		};

		return match mod_version
			.get_version()
			.cmp(cached_mod_version.get_version())
		{
			Ordering::Less => ModCacheStatus::NewerVersion,
			Ordering::Equal => ModCacheStatus::SameVersion,
			Ordering::Greater => ModCacheStatus::OlderVersion,
		};
	}

	pub async fn cache_mod<Download: ModVersionDownload>(&mut self, downloader: &Download) -> Result<PathBuf> {
		let mod_path = self.ensure_mod_folder(downloader)?;

		let mod_file_name = to_file_name(downloader);
		let mod_file_path = mod_path.join(Path::new(&mod_file_name));
		let manifest_path = ModManifest::create_manifest_path(mod_path, &mod_file_name)?;

		let mut writer = BufWriter::new(File::create(&mod_file_path)?);
		let stream = downloader.download().await?;
		pin!(stream);
		while let Some(item) = stream.next().await {
			let item = item?;
			writer.write_all(item.as_ref())?;
		}
		
		let writer = BufWriter::new(File::create(manifest_path)?);
		let manifest: ModManifest = downloader.into();
		serde_json::to_writer(writer, &manifest)?;

		self.cached_mods = calculate_cache(&self.cache_dir)?;

		Ok(mod_file_path)
	}

	#[allow(dead_code)]
	pub fn remove_cached_mod<MN: ModName>(&mut self, mod_name: &MN) -> Result<()>{
		let buf = self.get_mod_folder(mod_name)?;
		fs::remove_dir_all(buf)?;
		self.cached_mods = calculate_cache(&self.cache_dir)?;
		Ok(())
	}

	fn ensure_mod_folder<MN: ModName>(&self, mod_name: &MN) -> Result<PathBuf> {
		let mod_folder_name = to_folder_name(mod_name);
		let mod_path = self.cache_dir.join(mod_folder_name);
		if !mod_path.is_dir() {
			fs::create_dir(&mod_path)?;
		}
		Ok(mod_path)
	}

	fn get_mod_folder<MN: ModName>(&self, mod_version: &MN) -> Result<PathBuf> {
		let mod_folder_name = to_folder_name(mod_version);
		let mod_path = self.cache_dir.join(mod_folder_name);
		Ok(mod_path)
	}
}

fn calculate_cache<P: AsRef<Path>>(cache_path: P) -> Result<Vec<CachedMod>> {
	let mut cached_mods = Vec::new();
	for entry in fs::read_dir(cache_path)? {
		let entry = entry?;
		let path = entry.path();
		if !path.is_dir() {
			continue;
		}

		let cached_files = get_all_files(&path)?;
		let versions = clean_unmanaged_files_and_build_cache(cached_files)?;
		if versions.is_empty() {
			continue;
		}
		let name = versions
			.first()
			.map(|cmv| cmv.manifest.get_name().to_string())
			.context("Found no mod name")?;
		cached_mods.push(CachedMod::new(path, name, versions));
	}
	Ok(cached_mods)
}

fn clean_unmanaged_files_and_build_cache(mut vec: Vec<CacheFile>) -> Result<Vec<CachedModVersion>> {
	let mut to_keep = Vec::new();
	let mut cached_mods = Vec::new();
	for cached_file in vec.iter() {
		if !cached_file.is_manifest() {
			continue;
		}

		let Some(paired) = vec
			.iter()
			.find(|f| f.file_name == cached_file.file_name && f.file_ext != cached_file.file_ext)
		else {
			continue;
		};

		let file = File::open(&cached_file.path)?;
		let Ok(manifest) = serde_json::from_reader(BufReader::new(file)) else {
			eprintln!("Failed to parse: {}", cached_file.path.to_string_lossy());
			continue;
		};

		cached_mods.push(CachedModVersion {
			manifest,
			path: paired.path.clone(),
		});

		to_keep.push(cached_file.path.clone());
		to_keep.push(paired.path.clone());
	}
	while let Some(remove_index) = vec.iter().position(|cf| !to_keep.contains(&cf.path)) {
		let file = vec.swap_remove(remove_index);
		file.delete()?
	}

	Ok(cached_mods)
}

fn get_all_files(folder_path: &PathBuf) -> Result<Vec<CacheFile>> {
	let mut vec = Vec::new();
	for read in fs::read_dir(folder_path)? {
		let entry = read?;
		let string = entry.file_name();
		let (file_name, file_ext) =
			separate_file_and_ext(string.to_str().context("Found no filename")?)
				.map_err(|_| anyhow!("Failed to parse file name"))?;
		vec.push(CacheFile {
			file_name,
			file_ext,
			path: entry.path(),
		})
	}
	Ok(vec)
}

fn to_folder_name<MN: ModName>(mod_version: &MN) -> OsString {
	mod_version
		.get_name()
		.chars()
		.map(space_mapper)
		.collect::<String>()
		.into()
}

fn to_file_name<Download: ModVersionDownload>(mod_version: &Download) -> String {
	let string = create_mod_file_prefix(mod_version);
	format!("{string}_{}", mod_version.get_file_name())
}

fn create_mod_file_prefix<MV: ModVersion>(mod_version: &MV) -> String {
	mod_version
		.get_version()
		.to_string()
		.chars()
		.map(space_mapper)
		.collect::<String>()
}

fn separate_file_and_ext(file_name: &str) -> PResult<(String, Option<String>)> {
	let (remainder, separate): (&str, Vec<_>) =
		separated(0.., take_until(0.., "."), ".").parse_peek(file_name)?;

	if separate.is_empty() {
		return Ok((remainder.to_string(), None));
	}
	Ok((separate.join("."), Some(remainder.to_string())))
}

fn space_mapper(c: char) -> char {
	match c {
		' ' => '_',
		'-' => '_',
		_ => c,
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_file_parser_proper() {
		let result = separate_file_and_ext("1.0.0_maxloo2-betterkeys-updated.zip").unwrap();
		assert_eq!(
			result,
			(
				"1.0.0_maxloo2-betterkeys-updated".to_string(),
				Some(".zip".to_string())
			)
		);
	}

	#[test]
	fn test_file_parser_simple() {
		let result = separate_file_and_ext("foo").unwrap();
		assert_eq!(result, ("foo".to_string(), None));
	}
}
