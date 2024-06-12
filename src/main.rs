use std::fs;
use std::path::Path;
use std::time::Duration;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use indicatif::ProgressBar;

use crate::cache_mod_access::{CacheModAccess, ModCacheStatus};
use crate::configuration_access::ConfigurationAccess;
use crate::remote_mod_access::{ModKind, RemoteModAccess};
use crate::shared_traits::ModVersionDownload;
use crate::spt_access::SptAccess;
use crate::time_access::Time;

mod cache_mod_access;
mod configuration_access;
mod remote_mod_access;
mod shared_traits;
mod spt_access;
mod time_access;

const SERVER_FILE_NAME: &str = "Aki.Server.exe";

#[derive(Debug, Parser)]
#[command(name = "spt mod installer")]
#[command(about = "A mod installer managed by ControlFreak", long_about = None)]
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
	},
	#[command(arg_required_else_help = true)]
	Backup{
		backup_to: String
	},
	#[command(arg_required_else_help = true)]
	Restore{
		restore_from: String
	},
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

	const TEMP_PATH: &str = "./sptmm_tmp";
	fs::create_dir_all(TEMP_PATH)?;

	let downloader = RemoteModAccess::new();
	let mut file_man = CacheModAccess::build(TEMP_PATH)?;
	let cfg_man = ConfigurationAccess::new();
	let spt_access = SptAccess::new("./", Time::new());

	match args.command {
		Commands::Update { target } => {
			update(&downloader, &mut file_man, &cfg_man, &spt_access, target).await?
		}
		Commands::Backup{backup_to} => backup(&spt_access, &backup_to)?,
		Commands::Restore {restore_from} => restore(&spt_access, &restore_from)?,
	}

	Ok(())
}

async fn update(
	mod_downloader: &RemoteModAccess,
	file_man: &mut CacheModAccess,
	cfg_man: &ConfigurationAccess,
	installer: &SptAccess<Time>,
	target: UpdateTarget,
) -> Result<()> {
	let mod_cfg_file = "./spt_mods.json";
	let Some(mod_cfgs) = cfg_man.get_mods_from_path(mod_cfg_file)? else {
		println!("Found no mods at {}", mod_cfg_file);
		return Ok(());
	};

	for mod_cfg in mod_cfgs {
		let Some(mod_kind) = ModKind::parse(&mod_cfg.url, mod_cfg.github_pattern) else {
			continue;
		};

		let version_downloader = get_newest_release(mod_downloader, mod_kind).await?;

		let Some(cached_mod) = (match file_man.get_mod_status(&version_downloader) {
			ModCacheStatus::SameVersion => {
				println!("Current mod is same version");
				None
			}
			ModCacheStatus::NewerVersion => {
				println!("Current mod is newer version");
				None
			}
			ModCacheStatus::NotCached => Some(file_man.cache_mod(&version_downloader).await?),
			ModCacheStatus::OlderVersion => Some(file_man.cache_mod(&version_downloader).await?),
		}) else {
			continue;
		};

		match target {
			UpdateTarget::Client => installer.install_for_client(cached_mod)?,
			UpdateTarget::Server => {}
		}
	}
	Ok(())
}

fn restore(spt_access: &SptAccess<Time>, restore_from: &str) -> Result<()> {
	let bar = ProgressBar::new_spinner();
	bar.enable_steady_tick(Duration::from_millis(100));
	bar.set_message("Restoring mods and configurations");
	spt_access.restore_from(restore_from)?;
	bar.finish_with_message( format!("Backed up mods to: {restore_from}"));
	Ok(())
}

fn backup(spt_access: &SptAccess<Time>, backup_to_path: &str) -> Result<()>{
	let bar = ProgressBar::new_spinner();
	bar.enable_steady_tick(Duration::from_millis(100));
	bar.set_message("Backing up mods and configurations");
	spt_access.backup_to(backup_to_path)?;
	bar.finish_with_message( format!("Restored your files from: {backup_to_path}"));
	Ok(())
}

async fn get_newest_release(
	manager: &RemoteModAccess,
	mod_kind: ModKind,
) -> Result<impl ModVersionDownload> {
	let bar = ProgressBar::new_spinner();
	bar.enable_steady_tick(Duration::from_millis(100));
	bar.set_message("Finding newest mod");
	let downloader = manager.get_newest_release(mod_kind).await?;
	bar.finish_with_message("Found newest mod");
	Ok(downloader)
}
