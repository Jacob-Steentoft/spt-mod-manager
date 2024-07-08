use std::borrow::Cow;
use std::time::Duration;

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand, ValueEnum};
use indicatif::{ProgressBar, ProgressStyle};
use sptmm_lib::cache_access::ProjectAccess;
use sptmm_lib::configuration_access::ConfigurationAccess;
use sptmm_lib::remote_mod_access::{ModKind, RemoteModAccess};
use sptmm_lib::shared_traits::ModVersion;
use sptmm_lib::spt_access::{InstallTarget, SptAccess};
use sptmm_lib::time_access::Time;

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
	},
	#[command(arg_required_else_help = true)]
	Backup {
		backup_to: String,
	},
	#[command(arg_required_else_help = true)]
	Restore {
		restore_from: String,
	},
	CleanCache,
	RemoveMods,
}

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
enum UpdateTarget {
	Client,
	Server,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
	let args = Cli::parse();
	
	let root_path = "./";
	let project_access = ProjectAccess::new().map_err(|e| anyhow!(e))?;
	let mut remote_access = RemoteModAccess::init(&project_access).await?;
	let cfg_access = ConfigurationAccess::setup(root_path).await?;
	let spt_access = SptAccess::init(root_path, &project_access, Time::new())?;

	match args.command {
		Commands::Update {
			target
		} => {
			update(
				&mut remote_access,
				&cfg_access,
				&spt_access,
				target,
			)
			.await?
		}
		Commands::Backup { backup_to } => backup(&spt_access, &backup_to)?,
		Commands::Restore { restore_from } => restore(&spt_access, &restore_from)?,
		Commands::CleanCache => cleanup(&mut remote_access).await?,
		Commands::RemoveMods => remove_mods(&spt_access)?,
	}

	Ok(())
}

async fn cleanup(cache_access: &mut RemoteModAccess) -> Result<()> {
	cache_access.remove_cache().await
}

async fn update(
	remote_mod_access: &mut RemoteModAccess,
	cfg_man: &ConfigurationAccess,
	spt_access: &SptAccess<Time>,
	target: UpdateTarget,
) -> Result<()> {
	let mod_cfg = cfg_man.read_remote_mods().await?;

	for mod_cfg in mod_cfg.mods {
		let mod_url = mod_cfg.url;

		let mod_kind = match ModKind::parse(&mod_url, mod_cfg.github_pattern, mod_cfg.github_filter)
		{
			Ok(mod_kind) => mod_kind,
			Err(err) => {
				println!("Failed to parse '{mod_url}' with: {err}");
				continue;
			}
		};

		let bar = ProgressBar::new_spinner();
		bar.enable_steady_tick(Duration::from_millis(100));

		let cached_mod = match mod_cfg.version {
			None => {
				bar.set_message(format!("Finding newest version online for: {mod_url}"));
				let result = remote_mod_access.get_newest_release(mod_kind).await;
				match result {
					Ok(mod_version) => mod_version,
					Err(err) => {
						fail_with_error(bar, format!("Failed storing mod '{mod_url}' with error: {err}"));
						continue;
					}
				}
			}
			Some(version) => {
				bar.set_message(format!("Finding version '{version}' for: {mod_url}"));

				let option = match remote_mod_access
					.get_specific_version(mod_kind, &version)
					.await
				{
					Ok(mod_version) => mod_version,
					Err(err) => {
						fail_with_error(bar, format!("Failed to find versions for '{mod_url}' with error: {err}"));
						continue;
					}
				};

				let Some(cached_mod) = option else {
					fail_with_error(
						bar,
						format!("Failed to find version '{version}' for: {mod_url}"),
					);
					continue;
				};
				cached_mod
			}
		};
		
		if let Some(install_path) = mod_cfg.install_path {
			spt_access.install_mod_to_path(&cached_mod.path, install_path)?;
		} else {
			let install_target = match target {
				UpdateTarget::Client => InstallTarget::Client,
				UpdateTarget::Server => InstallTarget::Server,
			};
			if spt_access.is_same_installed_version(&cached_mod.path, &cached_mod, install_target)? {
				bar.finish_with_message(format!(
					"Version {} has already been installed for: {mod_url}", cached_mod.get_version()
				));
				continue;
			}
			bar.set_message(format!("Installing the newest version for: {mod_url}"));
			match spt_access.install_mod(&cached_mod.path, &cached_mod, install_target) {
				Ok(_) => {
					bar.finish_with_message(format!(
						"Installed version {} for: {mod_url}", cached_mod.get_version()
					));
				}
				Err(err) => fail_with_error(
					bar,
					format!("Failed to install '{mod_url}' with error: {err}"),
				),
			};
		};
	}
	Ok(())
}

fn remove_mods(spt_access: &SptAccess<Time>) -> Result<()>{
	spt_access.remove_all_mods()
}

fn restore(spt_access: &SptAccess<Time>, restore_from: &str) -> Result<()> {
	let bar = ProgressBar::new_spinner();
	bar.enable_steady_tick(Duration::from_millis(100));
	bar.set_message("Restoring mods and configurations");
	spt_access.restore_from(restore_from)?;
	bar.finish_with_message(format!("Restored your files from: {restore_from}"));
	Ok(())
}

fn backup(spt_access: &SptAccess<Time>, backup_to_path: &str) -> Result<()> {
	let bar = ProgressBar::new_spinner();
	bar.enable_steady_tick(Duration::from_millis(100));
	bar.set_message("Backing up mods and configurations");
	spt_access.backup_to(backup_to_path)?;
	bar.finish_with_message(format!("Backed up mods to: {backup_to_path}"));
	Ok(())
}

fn fail_with_error(bar: ProgressBar, msg: impl Into<Cow<'static, str>>) {
	bar.set_style(ProgressStyle::with_template("{spinner} {msg:.red}").unwrap());
	bar.finish_with_message(msg);
}
