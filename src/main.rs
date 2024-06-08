use std::cmp::Ordering;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use anyhow::Result;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand, ValueEnum};
use futures_core::Stream;
use indicatif::ProgressBar;
use versions::Versioning;

use crate::file_manager::{FileManager, ModCacheStatus};
use crate::mod_downloader::{ModDownloader, ModKind};

mod file_manager;
mod mod_downloader;

const SERVER_FILE_NAME: &str = "Aki.Server.exe";


pub trait ModName {
	fn get_name(&self) -> &str;

	fn is_same_name<Name: ModName>(&self, mod_name: &Name) -> bool;
}

pub trait ModVersion: ModName {
	fn get_version(&self) -> &Versioning;
	fn get_order<Version: ModVersion>(&self, mod_version: &Version) -> Ordering;
}

pub trait ModVersionDownload: ModVersion + Unpin {
	#[allow(async_fn_in_trait)]
	async fn download(&self) -> Result<impl Stream<Item = reqwest::Result<Bytes>>>;
	fn get_file_name(&self) -> &str;
	fn get_upload_date(&self) -> DateTime<Utc>;
}

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

	const TEMP_PATH: &str = "./sptmm_tmp";
	fs::create_dir_all(TEMP_PATH)?;

	let downloader = ModDownloader::new();
	let mut file_man = FileManager::build(TEMP_PATH)?;

	match args.command {
		Commands::Update { target } => update(&downloader, &mut file_man, target).await?,
	}

	Ok(())
}

async fn update(
	mod_downloader: &ModDownloader,
	file_manager: &mut FileManager,
	target: UpdateTarget,
) -> Result<()> {
	if target == UpdateTarget::Server {
		Command::new("docker").args(["stop", "fika"]).output()?;
	}

	let version_downloader = get_newest_release(
		mod_downloader,
		ModKind::SpTarkov {
			url: "https://hub.sp-tarkov.com/files/file/1963-better-keys-updated/".to_string(),
		},
	)
	.await?;

	match file_manager.get_mod_status(&version_downloader) {
		ModCacheStatus::SameVersion => {
			println!("Current mod is same version")
		}
		ModCacheStatus::NewerVersion => {
			println!("Current mod is newer version")
		}
		ModCacheStatus::NotCached => file_manager.cache_mod(&version_downloader).await?,
		ModCacheStatus::OlderVersion => file_manager.cache_mod(&version_downloader).await?,
	};

	if target == UpdateTarget::Server {
		Command::new("docker").args(["start", "fika"]).output()?;
	}
	Ok(())
}

async fn get_newest_release(
	manager: &ModDownloader,
	mod_kind: ModKind,
) -> Result<impl ModVersionDownload> {
	let bar = ProgressBar::new_spinner();
	bar.enable_steady_tick(Duration::from_millis(100));
	bar.set_message("Finding newest mod");
	let downloader = manager.get_newest_release(mod_kind).await?;
	bar.finish_with_message("Found newest mod");
	Ok(downloader)
}
