use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use anyhow::anyhow;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use versions::Versioning;
use crate::file_manager::separate_file_and_ext;
use crate::mod_downloader::ModDownloadVersion;
use crate::{ModName, ModVersion, ModVersionDownload};

#[derive(Serialize, Deserialize)]
pub struct ModManifest {
	name: String,
	version: Versioning,
	uploaded_at: DateTime<Utc>,
}

impl<Download: ModVersionDownload> From<&Download> for ModManifest {
	fn from(value: &Download) -> Self {
		Self {
			uploaded_at: value.get_upload_date(),
			name: value.get_name().to_string(),
			version: value.get_version().clone(),
		}
	}
}

impl ModManifest {
	pub fn is_mod_version(&self, mod_version: &ModDownloadVersion) -> bool {
		self.version == mod_version.version
	}

	pub fn create_manifest_path(mod_path: PathBuf, mod_file_name: &str) -> anyhow::Result<PathBuf> {
		let (manifest_file_name, _) = separate_file_and_ext(mod_file_name).map_err(|_| anyhow!("Failed to get file"))?;
		let manifest_file_name = format!("{}.manifest", manifest_file_name);
		let manifest_path = mod_path.join(Path::new(&manifest_file_name));
		Ok(manifest_path)
	}
}

impl ModName for ModManifest {
	fn get_name(&self) -> &str {
		&self.name
	}

	fn is_same_name<Name: ModName>(&self, mod_name: &Name) -> bool {
		self.name == mod_name.get_name()
	}
}

impl ModVersion for ModManifest {
	fn get_version(&self) -> &Versioning {
		&self.version
	}

	fn get_order<Version: ModVersion>(&self, mod_version: &Version) -> Ordering {
		self.version.cmp(mod_version.get_version())
	}
}