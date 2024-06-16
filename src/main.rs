use std::fs;
use std::path::Path;
use std::time::Duration;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use indicatif::ProgressBar;

use crate::cache_mod_access::{CacheModAccess, ModCacheStatus};
use crate::configuration_access::ConfigurationAccess;
use crate::remote_mod_access::{ModKind, RemoteModAccess};
use crate::spt_access::{InstallTarget, SptAccess};
use crate::time_access::Time;

mod cache_mod_access;
mod configuration_access;
mod remote_mod_access;
mod shared_traits;
mod spt_access;
mod time_access;

const SERVER_FILE_NAME: &str = "Aki.Server.exe";
const TEMP_PATH: &str = "./sptmm_tmp";

#[derive(Debug, Parser)]
#[command(name = "spt mod manager")]
#[command(about = "A mod manager created by ControlFreak for SPTarkov", long_about = None)]
struct Cli {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
	#[command(arg_required_else_help = true)]
	Update {
		#[arg(required = true)]
		target: UpdateTarget,
		configuration_path: Option<String>,
	},
	#[command(arg_required_else_help = true)]
	Backup { backup_to: String },
	#[command(arg_required_else_help = true)]
	Restore { restore_from: String },
	Cleanup,
}

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
enum UpdateTarget {
	Client,
	Server,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
	if !Path::new(&format!("./{SERVER_FILE_NAME}")).exists() {
		eprintln!("ERROR: Could not find {SERVER_FILE_NAME} in the current folder");
		return Ok(());
	}
	let args = Cli::parse();

	fs::create_dir_all(TEMP_PATH)?;

	let remote_access = RemoteModAccess::new();
	let mut cache_mod_access = CacheModAccess::build(TEMP_PATH)?;
	let cfg_access = ConfigurationAccess::new();
	let spt_access = SptAccess::init("./", Time::new())?;

	match args.command {
		Commands::Update {
			target,
			configuration_path,
		} => {
			update(
				&remote_access,
				&mut cache_mod_access,
				&cfg_access,
				&spt_access,
				target,
				configuration_path,
			)
			.await?
		}
		Commands::Backup { backup_to } => backup(&spt_access, &backup_to)?,
		Commands::Restore { restore_from } => restore(&spt_access, &restore_from)?,
		Commands::Cleanup => cleanup(&mut cache_mod_access)?,
	}

	Ok(())
}

fn cleanup(cache_access: &mut CacheModAccess) -> Result<()> {
	cache_access.remove_cache()
}

async fn update(
	remote_mod_access: &RemoteModAccess,
	cache_mod_access: &mut CacheModAccess,
	cfg_man: &ConfigurationAccess,
	spt_access: &SptAccess<Time>,
	target: UpdateTarget,
	configuration_path: Option<String>,
) -> Result<()> {
	let mod_cfg_file = configuration_path.unwrap_or("./spt_mods.json".to_string());
	let Some(mod_cfg) = cfg_man.get_mods_from_path(&mod_cfg_file)? else {
		println!("Found no mod config at: {mod_cfg_file}");
		return Ok(());
	};
	println!("Found mod config at: {mod_cfg_file}");

	for mod_cfg in mod_cfg {
		let mod_url = mod_cfg.url;
		
		let mod_kind = match ModKind::parse(&mod_url, mod_cfg.github_pattern) {
			Ok(mod_kind) => mod_kind,
			Err(err) => {
				println!("Failed to parse mod with: {err}");
				continue;
			}
		};
		
		let bar = ProgressBar::new_spinner();
		bar.enable_steady_tick(Duration::from_millis(100));

		let mod_downloader = match mod_cfg.version {
			None => {
				bar.set_message(format!("Finding newest version online for: {mod_url}"));
				let version_downloader = remote_mod_access.get_newest_release(mod_kind).await?;
				match cache_mod_access.get_status(&version_downloader) {
					ModCacheStatus::SameVersion => None,
					ModCacheStatus::NewerVersion => None,
					ModCacheStatus::NotCached | ModCacheStatus::OlderVersion => {
						Some(version_downloader)
					}
				}
			}
			Some(version) => {
				let option = remote_mod_access
					.get_specific_version(mod_kind, &version)
					.await?;
				let Some(version_downloader) = option else {
					bar.finish_with_message(format!(
						"Did not find version: {}, for: {mod_url}",
						version
					));
					continue;
				};
				// TODO: Migrate cache to remote
				match cache_mod_access.get_status(&version_downloader) {
					ModCacheStatus::SameVersion => None,
					ModCacheStatus::NewerVersion
					| ModCacheStatus::NotCached
					| ModCacheStatus::OlderVersion => Some(version_downloader),
				}
			}
		};

		let mod_to_install = match mod_downloader {
			None => {
				// TODO: Verify locally installed mod
				bar.finish_with_message(format!("Newest version already installed for: {mod_url}"));
				continue;
			}
			Some(version_downloader) => {
				bar.set_message(format!("Downloading the newest version for: {mod_url}"));
				cache_mod_access.cache_mod(&version_downloader).await?
			}
		};

		bar.set_message(format!("Installing the newest version for: {mod_url}"));
		if let Some(install_path) = mod_cfg.install_path {
			spt_access.install_mod_to_path(mod_to_install, install_path.into())?;
		} else {
			match target {
				UpdateTarget::Client => {
					spt_access.install_mod(mod_to_install, InstallTarget::Client)?
				}
				UpdateTarget::Server => {
					spt_access.install_mod(mod_to_install, InstallTarget::Server)?
				}
			}
		};

		bar.finish_with_message(format!(
			"The newest version has been installed for: {mod_url}"
		));
	}
	Ok(())
}

fn restore(spt_access: &SptAccess<Time>, restore_from: &str) -> Result<()> {
	let bar = ProgressBar::new_spinner();
	bar.enable_steady_tick(Duration::from_millis(100));
	bar.set_message("Restoring mods and configurations");
	spt_access.restore_from(restore_from)?;
	bar.finish_with_message(format!("Backed up mods to: {restore_from}"));
	Ok(())
}

fn backup(spt_access: &SptAccess<Time>, backup_to_path: &str) -> Result<()> {
	let bar = ProgressBar::new_spinner();
	bar.enable_steady_tick(Duration::from_millis(100));
	bar.set_message("Backing up mods and configurations");
	spt_access.backup_to(backup_to_path)?;
	bar.finish_with_message(format!("Restored your files from: {backup_to_path}"));
	Ok(())
}
