use std::path::Path;

use directories_next::ProjectDirs;

#[derive(Debug, Clone)]
pub struct ProjectAccess {
	project_dirs: ProjectDirs,
}

impl ProjectAccess {
	pub fn new() -> Result<Self, String> {
		let Some(project_dirs) = ProjectDirs::from("net", "steentoft", "sptmm") else {
			return Err("Failed to create project directory".to_string());
		};
		Ok(Self { project_dirs })
	}

	pub fn from(path: impl AsRef<Path>) -> Result<Self, String> {
		let Some(project_dirs) = ProjectDirs::from_path(path.as_ref().to_path_buf()) else {
			return Err("Failed to create project directory".to_string());
		};
		Ok(Self { project_dirs })
	}

	pub fn cache_root(&self) -> &Path {
		self.project_dirs.cache_dir()
	}

	pub fn config_root(&self) -> &Path {
		self.project_dirs.config_dir()
	}
}
