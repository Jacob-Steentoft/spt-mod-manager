use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use versions::Versioning;

#[derive(PartialEq, Debug)]
pub struct ModConfiguration {
	pub url: String,
	pub version: Option<Versioning>,
	pub github_pattern: Option<String>,
	pub install_path: Option<String>,
	pub github_filter: Option<String>,
}

#[derive(Deserialize, Serialize)]
struct ModConfigurationRaw {
	url: String,
	version: Option<String>,
	github_pattern: Option<String>,
	install_path: Option<String>,
	github_filter: Option<String>,
}

impl TryFrom<ModConfigurationRaw> for ModConfiguration {
	type Error = anyhow::Error;

	fn try_from(value: ModConfigurationRaw) -> std::result::Result<Self, Self::Error> {
		let version = if let Some(version) = value.version {
			Some(Versioning::try_from(version.as_str())?)
		} else {
			None
		};

		Ok(Self {
			url: value.url,
			install_path: value.install_path,
			github_pattern: value.github_pattern,
			github_filter: value.github_filter,
			version,
		})
	}
}

pub struct ConfigurationAccess {}

impl ConfigurationAccess {
	pub fn new() -> Self {
		Self {}
	}
	pub fn get_mods_from_path<P: AsRef<Path>>(
		&self,
		mod_cfg_path: P,
	) -> Result<Option<Vec<ModConfiguration>>> {
		let path = mod_cfg_path.as_ref();
		if !path.is_file() {
			return Ok(None);
		}

		let reader = BufReader::new(File::open(path)?);
		let raw_cfgs: Vec<ModConfigurationRaw> = serde_json::from_reader(reader)?;

		let mut cfgs = Vec::new();
		for x in raw_cfgs {
			cfgs.push(ModConfiguration::try_from(x)?)
		}

		Ok(Some(cfgs))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	//TODO: More tests please :)

	#[test]
	fn integration_test_get_mods_from_path() {
		let path = "./test_data/cfg_mods.json";
		let option = ConfigurationAccess::new().get_mods_from_path(path).unwrap();

		let vec1 = vec![ModConfiguration {
			url: "https://github.com/test/mactest/".to_string(),
			version: None,
			github_pattern: None,
			install_path: None,
			github_filter: None,
		}];
		assert_eq!(option, Some(vec1));
	}
}
