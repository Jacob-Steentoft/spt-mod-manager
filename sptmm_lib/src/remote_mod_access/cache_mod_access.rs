use std::cmp::Ordering;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use tokio::fs;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use versions::Versioning;
use winnow::combinator::separated;
use winnow::prelude::*;
use winnow::token::take_until;
use winnow::PResult;

use crate::path_access::PathAccess;
use crate::remote_mod_access::cache_mod_access::cached_mod::CachedMod;
pub use crate::remote_mod_access::cache_mod_access::cached_mod_version::CachedModVersion;
use crate::remote_mod_access::cache_mod_access::mod_manifest::ModManifest;
use crate::remote_mod_access::ModKind;
use crate::shared_traits::{ModName, ModVersion, ModVersionDownload};

mod cached_mod;
mod cached_mod_version;
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
	async fn delete(self) -> Result<()> {
		fs::remove_file(self.path).await?;
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
	pub async fn init(project: &PathAccess) -> Result<Self> {
		let cache_dir = project.cache_root().join("remote");
		fs::create_dir_all(&cache_dir).await?;
		let cached_mods = calculate_cache(&cache_dir).await?;
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

	pub fn get_cached_mod<Version: ModVersion>(
		&self,
		mod_version: &Version,
	) -> Option<&CachedModVersion> {
		self.cached_mods
			.iter()
			.find(|x| x.is_same_name(mod_version))
			.and_then(|m| m.get_version(mod_version.get_version()))
	}

	pub fn get_cached_mod_from_kind(
		&self,
		mod_kind: &ModKind,
		version: &Versioning,
	) -> Option<&CachedModVersion> {
		self.cached_mods
			.iter()
			.find(|x| x.get_mod_kind() == mod_kind)
			.and_then(|m| m.get_version(version))
	}

	pub async fn cache_mod<Download: ModVersionDownload>(
		&mut self,
		downloader: Download,
		mod_kind: ModKind,
	) -> Result<&CachedModVersion> {
		let mod_path = self.ensure_mod_folder(&downloader).await?;

		let mod_file_name = to_file_name(&downloader);
		let mod_file_path = mod_path.join(Path::new(&mod_file_name));
		let manifest_path = ModManifest::create_manifest_path(mod_path, &mod_file_name)?;

		let mut archive_file = File::create(&mod_file_path).await?;
		let stream = downloader.download().await?;
		archive_file.write_all(stream.as_ref()).await?;

		let mut manifest_file = File::create(manifest_path).await?;
		let manifest = ModManifest::new(
			downloader.get_upload_date(),
			downloader.get_name().to_string(),
			downloader.get_version().clone(),
			mod_kind,
		);
		let buffer = serde_json::to_vec(&manifest)?;
		manifest_file.write_all(&buffer).await?;

		self.cached_mods = calculate_cache(&self.cache_dir).await?;

		let version = self
			.cached_mods
			.iter()
			.find_map(|x| x.get_version(downloader.get_version()))
			.context("Failed to find cached version")?;

		Ok(version)
	}

	pub async fn remove_cache(&mut self) -> Result<()> {
		let mut read = fs::read_dir(&self.cache_dir).await?;
		while let Some(entry) = read.next_entry().await? {
			let path = entry.path();
			if path.is_dir() {
				fs::remove_dir_all(path).await?;
			} else if path.is_file() {
				fs::remove_file(path).await?
			}
		}

		self.cached_mods = calculate_cache(&self.cache_dir).await?;
		Ok(())
	}

	async fn ensure_mod_folder<MN: ModName>(&self, mod_name: &MN) -> Result<PathBuf> {
		let mod_folder_name = mod_name.to_file_name();
		let mod_path = self.cache_dir.join(mod_folder_name);
		if !mod_path.is_dir() {
			fs::create_dir(&mod_path).await?;
		}
		Ok(mod_path)
	}
}

async fn calculate_cache<P: AsRef<Path>>(cache_path: P) -> Result<Vec<CachedMod>> {
	let mut cached_mods = Vec::new();
	let mut read = fs::read_dir(&cache_path).await?;
	while let Some(entry) = read.next_entry().await? {
		let path = entry.path();
		if !path.is_dir() {
			continue;
		}

		let cached_files = get_all_files(&path).await?;
		let versions = clean_unmanaged_files_and_build_cache(cached_files).await?;
		if versions.is_empty() {
			continue;
		}
		let (name, mod_kind) = versions
			.first()
			.map(|cmv| {
				(
					cmv.manifest.get_name().to_string(),
					cmv.manifest.get_mod_kind().clone(),
				)
			})
			.context("Found no mod name")?;
		cached_mods.push(CachedMod::new(name, versions, mod_kind));
	}
	Ok(cached_mods)
}

async fn clean_unmanaged_files_and_build_cache(
	mut vec: Vec<CacheFile>,
) -> Result<Vec<CachedModVersion>> {
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

		let mut file = File::open(&cached_file.path).await?;
		let mut buffer = Vec::new();
		file.read_to_end(&mut buffer).await?;
		let Ok(manifest) = serde_json::from_slice(&buffer) else {
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
		file.delete().await?
	}

	Ok(cached_mods)
}

async fn get_all_files(folder_path: &PathBuf) -> Result<Vec<CacheFile>> {
	let mut vec = Vec::new();
	let mut read = fs::read_dir(&folder_path).await?;
	while let Some(entry) = read.next_entry().await? {
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

fn to_file_name<Download: ModVersionDownload>(mod_version: &Download) -> String {
	format!(
		"{}_{}",
		mod_version.to_file_version(),
		mod_version.get_file_name()
	)
}

fn separate_file_and_ext(file_name: &str) -> PResult<(String, Option<String>)> {
	let (remainder, separate): (&str, Vec<_>) =
		separated(0.., take_until(0.., "."), ".").parse_peek(file_name)?;

	if separate.is_empty() {
		return Ok((remainder.to_string(), None));
	}
	Ok((separate.join("."), Some(remainder.to_string())))
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
