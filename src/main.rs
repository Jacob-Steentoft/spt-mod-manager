use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use indicatif::ProgressBar;

use crate::mod_downloader::{ModKind, ModManager, ModVersionDownloader};

mod mod_downloader;

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
	const TEMP_PATH: &str = "./tmp";
	fs::create_dir_all(TEMP_PATH)?;
	
	let current_mods: Vec<_> = fs::read_dir(TEMP_PATH)?
		.filter(|entry| {
			entry
				.as_ref()
				.is_ok_and(|entry| entry.file_type().is_ok_and(|ft| ft.is_file()))
		})
		.flatten()
		.collect();

	let downloader = check_newest_release(manager,ModKind::SpTarkov { url: "https://hub.sp-tarkov.com/files/file/1963-better-keys-updated/".to_string() }).await?;
	
	let string = OsString::from(&downloader.mod_version().file_name);
	if current_mods
		.iter()
		.any(|entry| entry.file_name().eq(&string))
	{
		downloader.download(TEMP_PATH).await?;
	};

	if target == UpdateTarget::Server {
		Command::new("docker").args(["start", "fika"]).output()?;
	}
	Ok(())
}

async fn check_newest_release(manager: &ModManager, mod_kind: ModKind) -> Result<ModVersionDownloader>{
	let bar = ProgressBar::new_spinner();
	bar.enable_steady_tick(Duration::from_millis(100));
	bar.set_message("Finding newest mod");
	let downloader = manager.get_newest_release(mod_kind).await?;
	bar.finish_with_message("Found newest mod");
	Ok(downloader)
}