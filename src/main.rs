use std::fs;
use std::fs::File;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use indicatif::ProgressBar;

use crate::mod_downloader::{ModKind, ModManager, ModVersionDownloader};

mod mod_downloader;
mod file_manager;

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

	

	let manager = ModManager::new();

	match args.command {
		Commands::Update { target } => update(&manager, target).await?,
	}

	Ok(())
}

async fn update(manager: &ModManager, target: UpdateTarget) -> Result<()> {
	if target == UpdateTarget::Server {
		Command::new("docker").args(["stop", "fika"]).output()?;
	}
	const TEMP_PATH: &str = "./sptmm_tmp";
	fs::create_dir_all(TEMP_PATH)?;
	

	let downloader = get_newest_release(manager, ModKind::SpTarkov { url: "https://hub.sp-tarkov.com/files/file/1963-better-keys-updated/".to_string() }).await?;

	let mut result = File::create(format!("{}/{}", TEMP_PATH, downloader.mod_version().file_name))?;

	downloader.download(&mut result).await?;

	if target == UpdateTarget::Server {
		Command::new("docker").args(["start", "fika"]).output()?;
	}
	Ok(())
}

async fn get_newest_release(manager: &ModManager, mod_kind: ModKind) -> Result<ModVersionDownloader>{
	let bar = ProgressBar::new_spinner();
	bar.enable_steady_tick(Duration::from_millis(100));
	bar.set_message("Finding newest mod");
	let downloader = manager.get_newest_release(mod_kind).await?;
	bar.finish_with_message("Found newest mod");
	Ok(downloader)
}