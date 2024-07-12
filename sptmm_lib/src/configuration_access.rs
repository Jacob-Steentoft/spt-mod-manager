use std::path::PathBuf;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use tokio::fs::OpenOptions;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use versions::Versioning;

use crate::path_access::PathAccess;

#[derive(PartialEq, Debug, Clone)]
pub struct ModConfiguration {
	pub spt_version: Versioning,
	pub mods: Vec<ModVersionConfiguration>,
}

#[derive(PartialEq, Debug, Clone)]
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

impl From<ModVersionConfiguration> for ModVersionConfigurationRaw {
	fn from(value: ModVersionConfiguration) -> Self {
		Self {
			url: value.url,
			install_path: value.install_path,
			github_pattern: value.github_pattern,
			github_filter: value.github_filter,
			version: value.version.map(|t| t.to_string()),
		}
	}
}

impl From<ModConfiguration> for ModConfigurationRaw {
	fn from(value: ModConfiguration) -> Self {
		Self {
			spt_version: value.spt_version,
			mods: value.mods.into_iter().map(|x| x.into()).collect(),
		}
	}
}

#[derive(Debug, Clone)]
pub struct ConfigurationAccess {
	mod_cfg_path: PathBuf,
}

impl ConfigurationAccess {
	pub async fn init(path: &PathAccess) -> Result<Self> {
		let root_path = path.spt_root();
		if !root_path.is_dir() {
			return Err(anyhow!("Root folder must be a directory"));
		}
		let mod_cfg_path = root_path.join("spt_mods.json");

		Ok(Self { mod_cfg_path })
	}
	pub async fn read_remote_mods(&self) -> Result<ModConfiguration> {
		let mut buffer = Vec::new();
		OpenOptions::new()
			.read(true)
			.open(&self.mod_cfg_path)
			.await?
			.read_to_end(&mut buffer)
			.await?;

		let raw_cfgs: ModConfigurationRaw = serde_json::from_slice(&buffer)?;

		let mut mods = Vec::new();
		for x in raw_cfgs.mods {
			mods.push(ModVersionConfiguration::try_from(x)?)
		}

		Ok(ModConfiguration {
			mods,
			spt_version: raw_cfgs.spt_version,
		})
	}

	pub async fn write_remote_mods(&self, mod_configuration: &ModConfiguration) -> Result<()> {
		let cfg: ModConfigurationRaw = mod_configuration.clone().into();
		let buffer = serde_json::to_vec(&cfg)?;
		let mut file = OpenOptions::new()
			.create(true)
			.truncate(true)
			.open(&self.mod_cfg_path)
			.await?;
		file.write_all(&buffer).await?;
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	//TODO: More tests please :)

	#[tokio::test]
	async fn integration_test_get_mods_from_path() {
		let path_access = PathAccess::from("./test_data/", "./test_data/").unwrap();
		let option = ConfigurationAccess::init(&path_access)
			.await
			.unwrap()
			.read_remote_mods()
			.await
			.unwrap();

		let cfg = ModConfiguration {
			mods: vec![ModVersionConfiguration {
				url: "https://github.com/test/mactest/".to_string(),
				version: None,
				github_pattern: None,
				install_path: None,
				github_filter: None,
			}],
			spt_version: Versioning::Ideal("3.8.3".parse().unwrap()),
		};
		assert_eq!(option, cfg);
	}
}
