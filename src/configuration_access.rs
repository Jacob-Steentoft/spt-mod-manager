use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use serde::{Deserialize, Serialize};
use versions::Version;
use anyhow::Result;

#[derive(Deserialize, Serialize)]
pub struct ModConfiguration{
	pub url: String,
	pub version: Option<Version>,
	pub github_pattern: Option<String>
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