use std::cmp::Ordering;
use std::path::PathBuf;
use versions::Versioning;
use crate::file_manager::ModManifest;
use crate::{ModName, ModVersion};

pub struct CachedModVersion {
	pub path: PathBuf,
	pub manifest: ModManifest,
}

impl PartialEq<Self> for CachedModVersion {
	fn eq(&self, other: &Self) -> bool {
		self.path == other.path
	}
}

impl Eq for CachedModVersion {}

impl PartialOrd for CachedModVersion {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl Ord for CachedModVersion {
	fn cmp(&self, other: &Self) -> Ordering {
		self.manifest.get_order(other)
	}
}

impl ModName for CachedModVersion {
	fn get_name(&self) -> &str {
		self.manifest.get_name()
	}

	fn is_same_name<Name: ModName>(&self, mod_name: &Name) -> bool {
		self.manifest.get_name() == mod_name.get_name()
	}
}

impl ModVersion for CachedModVersion {
	fn get_version(&self) -> &Versioning {
		self.manifest.get_version()
	}

	fn get_order<Version: ModVersion>(&self, rhs: &Version) -> Ordering {
		self.manifest.get_order(rhs)
	}
}