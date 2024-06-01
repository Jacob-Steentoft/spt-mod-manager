use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand, ValueEnum};
use dircpy::CopyBuilder;
use git2::build::CheckoutBuilder;
use git2::Repository;
use indicatif::ProgressBar;

mod mod_manager;

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

fn main() -> Result<()> {
	if !Path::new(&format!("./{SERVER_FILE_NAME}")).exists() {
		eprintln!("ERROR: Could not find {SERVER_FILE_NAME} in the current folder");
		return Ok(());
	}
	let args = Cli::parse();

	match args.command {
		Commands::Update { target } => update(target)?,
	}

	Ok(())
}

fn update(target: UpdateTarget) -> Result<()> {
	if target == UpdateTarget::Server {
		Command::new("docker").args(["stop", "fika"]).output()?;
	}
	const TEMP_PATH: &str = "./tmp";

	fs::create_dir_all(TEMP_PATH)?;

	download_repo(TEMP_PATH)?;

	merge_mods(TEMP_PATH, "./")?;

	if target == UpdateTarget::Server {
		Command::new("docker").args(["start", "fika"]).output()?;
	}
	Ok(())
}

fn merge_mods(source_dir: &str, target_dir: &str) -> Result<()> {
	let bar = ProgressBar::new_spinner();
	bar.enable_steady_tick(Duration::from_millis(100));
	bar.set_message(format!("Merging mods into: {target_dir}"));
	copy_dir_all(source_dir, target_dir)?;
	bar.finish_with_message("Merged mods");
	Ok(())
}

fn download_repo(repo_dir: &str) -> Result<()> {
	let bar = ProgressBar::new_spinner();
	bar.enable_steady_tick(Duration::from_millis(100));
	bar.set_message("Downloading mods");

	match Repository::open(repo_dir) {
		Ok(repo) => fast_forward(repo, "master")?,
		Err(_) => {
			Repository::clone("https://github.com/Jacob-Steentoft/SPT.git", repo_dir)?;
		}
	};

	bar.finish_with_message("Downloaded mods");
	Ok(())
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<()> {
	CopyBuilder::new(src, dst)
		.overwrite_if_newer(true)
		.overwrite_if_size_differs(true)
		.with_exclude_filter(".gitignore")
		.run()?;
	Ok(())
}

fn fast_forward(repo: Repository, branch: &str) -> Result<()> {
	repo.find_remote("origin")?.fetch(&[branch], None, None)?;

	let fetch_head = repo.find_reference("FETCH_HEAD")?;
	let fetch_commit = repo.reference_to_annotated_commit(&fetch_head)?;
	let (analysis, _) = repo.merge_analysis(&[&fetch_commit])?;
	if analysis.is_up_to_date() {
		Ok(())
	} else if analysis.is_fast_forward() {
		let ref_name = format!("refs/heads/{}", branch);
		let mut reference = repo.find_reference(&ref_name)?;
		reference.set_target(fetch_commit.id(), "Fast-Forward")?;
		repo.set_head(&ref_name)?;
		repo.checkout_head(Some(CheckoutBuilder::default().force()))?;
		Ok(())
	} else {
		Err(anyhow!("Fast-forward only!"))
	}
}


