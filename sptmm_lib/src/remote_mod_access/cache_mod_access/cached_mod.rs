use versions::Versioning;
use crate::remote_mod_access::cache_mod_access::cached_mod_version::CachedModVersion;
use crate::shared_traits::{ModName, ModVersion};

pub struct CachedMod {
	name: String,
	versions: Vec<CachedModVersion>,
}

impl CachedMod {
	pub(crate) fn new(name: String, versions: Vec<CachedModVersion>,) -> Self{
		Self{
			name,
			versions,
		}
	}
	pub fn get_newest(&self) -> Option<&CachedModVersion> {
		self.versions.iter().max()
	}
	
	pub fn get_version(&self, version: &Versioning) -> Option<&CachedModVersion> {
		self.versions.iter().find(|x| x.get_version() == version)
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