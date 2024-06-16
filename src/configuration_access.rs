use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use versions::Versioning;

#[derive(Deserialize, Serialize, PartialEq, Debug)]
pub struct ModConfiguration{
	pub url: String,
	pub version: Option<Versioning>,
	pub github_pattern: Option<String>,
	pub install_path: Option<String>,
}

pub struct ConfigurationAccess {
	
}

impl ConfigurationAccess {
	pub fn new() -> Self{
		Self{}
	}
	pub fn get_mods_from_path<P: AsRef<Path>>(&self, mod_cfg_path: P) -> Result<Option<Vec<ModConfiguration>>>{
		let path = mod_cfg_path.as_ref();
		if !path.is_file() {
			return Ok(None)
		}

		let reader = BufReader::new(File::open(path)?);
		let result : Vec<ModConfiguration> = serde_json::from_reader(reader)?;
		Ok(Some(result))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn integration_test_get_mods_from_path() {
		let path = "./test_data/cfg_mods.json";
		let option = ConfigurationAccess::new().get_mods_from_path(path).unwrap();

		let vec1 = vec!(ModConfiguration {
			url: "https://github.com/test/mactest/".to_string(),
			version: None,
			github_pattern: None,
			install_path: None,
		});
		assert_eq!(option, Some(vec1));
	}
}
