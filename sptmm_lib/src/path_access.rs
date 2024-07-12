use directories_next::ProjectDirs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct PathAccess {
	project_dirs: ProjectDirs,
	spt_root: PathBuf,
}

impl PathAccess {
	pub fn new(spt_path: impl AsRef<Path>) -> Result<Self, String> {
		let Some(project_dirs) = ProjectDirs::from("net", "steentoft", "sptmm") else {
			return Err("Failed to create project directory".to_string());
		};
		Ok(Self {
			project_dirs,
			spt_root: spt_path.as_ref().into(),
		})
	}

	pub fn from(
		project_path: impl AsRef<Path>,
		spt_path: impl AsRef<Path>,
	) -> Result<Self, String> {
		let Some(project_dirs) = ProjectDirs::from_path(project_path.as_ref().to_path_buf()) else {
			return Err("Failed to create project directory".to_string());
		};
		Ok(Self {
			project_dirs,
			spt_root: spt_path.as_ref().into(),
		})
	}

	pub fn cache_root(&self) -> &Path {
		self.project_dirs.cache_dir()
	}

	pub fn config_root(&self) -> &Path {
		self.project_dirs.config_dir()
	}

	pub fn spt_root(&self) -> &Path {
		&self.spt_root
	}
}
