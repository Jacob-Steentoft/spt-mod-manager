use std::path::PathBuf;
use crate::cache_mod_access::cached_mod_version::CachedModVersion;
use crate::shared_traits::ModName;

pub struct CachedMod {
	path: PathBuf,
	name: String,
	versions: Vec<CachedModVersion>,
}

impl CachedMod {
	pub(super) fn new(path: PathBuf, name: String, versions: Vec<CachedModVersion>,) -> Self{
		Self{
			path,
			name,
			versions,
		}
	}
	pub fn get_newest(&self) -> Option<&CachedModVersion> {
		self.versions.iter().max()
	}
}

impl ModName for CachedMod {
	fn get_name(&self) -> &str {
		self.name.as_str()
	}

	fn is_same_name<Name: ModName>(&self, mod_name: &Name) -> bool {
		self.name == mod_name.get_name()
	}
}