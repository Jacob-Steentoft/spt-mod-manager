use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use versions::Versioning;

#[derive(PartialEq, Debug)]
pub struct ModConfiguration {
	pub spt_version: Versioning,
	pub mods: Vec<ModVersionConfiguration>,
}

#[derive(PartialEq, Debug)]
pub struct ModVersionConfiguration {
	pub url: String,
	pub version: Option<Versioning>,
	pub github_pattern: Option<String>,
	pub install_path: Option<String>,
	pub github_filter: Option<String>,
}

#[derive(Deserialize, Serialize)]
struct ModConfigurationRaw {
	#[serde(deserialize_with = "Versioning::deserialize_pretty")]
	spt_version: Versioning,
	mods: Vec<ModVersionConfigurationRaw>,
}
#[derive(Deserialize, Serialize)]
struct ModVersionConfigurationRaw {
	url: String,
	version: Option<String>,
	github_pattern: Option<String>,
	install_path: Option<String>,
	github_filter: Option<String>,
}

impl TryFrom<ModVersionConfigurationRaw> for ModVersionConfiguration {
	type Error = anyhow::Error;

	fn try_from(value: ModVersionConfigurationRaw) -> std::result::Result<Self, Self::Error> {
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
	) -> Result<Option<ModConfiguration>> {
		let path = mod_cfg_path.as_ref();
		if !path.is_file() {
			return Ok(None);
		}

		let reader = BufReader::new(File::open(path)?);
		let raw_cfgs: ModConfigurationRaw = serde_json::from_reader(reader)?;

		let mut mods = Vec::new();
		for x in raw_cfgs.mods {
			mods.push(ModVersionConfiguration::try_from(x)?)
		}
		
		Ok(Some(ModConfiguration {
			mods,
			spt_version: raw_cfgs.spt_version
		}))
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

		let cfg = ModConfiguration{
			mods: vec![ModVersionConfiguration {
				url: "https://github.com/test/mactest/".to_string(),
				version: None,
				github_pattern: None,
				install_path: None,
				github_filter: None,
			}],
			spt_version: Versioning::Ideal("3.8.3".parse().unwrap())
		}
		 ;
		assert_eq!(option, Some(cfg));
	}
}
