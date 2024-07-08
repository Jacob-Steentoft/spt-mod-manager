use crate::remote_mod_access::cache_mod_access::separate_file_and_ext;
use crate::remote_mod_access::ModKind;
use crate::shared_traits::{ModName, ModVersion};
use anyhow::anyhow;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use versions::Versioning;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModManifest {
	name: String,
	version: Versioning,
	uploaded_at: DateTime<Utc>,
	mod_kind: ModKind,
}

impl ModManifest {
	pub fn new(
		uploaded_at: DateTime<Utc>,
		name: String,
		version: Versioning,
		mod_kind: ModKind,
	) -> Self {
		Self {
			uploaded_at,
			name,
			version,
			mod_kind,
		}
	}
	pub fn create_manifest_path(mod_path: PathBuf, mod_file_name: &str) -> anyhow::Result<PathBuf> {
		let (manifest_file_name, _) =
			separate_file_and_ext(mod_file_name).map_err(|_| anyhow!("Failed to get file"))?;
		let manifest_file_name = format!("{}.manifest", manifest_file_name);
		let manifest_path = mod_path.join(Path::new(&manifest_file_name));
		Ok(manifest_path)
	}
	
	pub fn get_mod_kind(&self) -> &ModKind{
		&self.mod_kind
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
