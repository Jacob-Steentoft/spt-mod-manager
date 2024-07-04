use iced::*;
use serde::{Deserialize, Serialize};
use sptmm_lib::configuration_access::ModVersionConfiguration;
use sptmm_lib::spt_access::SptAccess;
use sptmm_lib::time_access::Time;

fn main() {
	println!("Hello, world!");
}

#[derive(Default, Debug)]
enum RemoteMods{
	#[default]
	Loading,
	Loaded(State)
}

#[derive(Debug)]
struct State {
	remote_client: SptAccess<Time>,
	tasks: Vec<ModVersionConfiguration>,
	dirty: bool,
	saving: bool,
}

#[derive(Debug, Clone)]
enum Message {
	Loaded(Result<SavedState, LoadError>),
	Saved(Result<(), SaveError>),
	InputChanged(String),
	CreateTask,
	FilterChanged(Filter),
	TaskMessage(usize, TaskMessage),
	TabPressed { shift: bool },
	ToggleFullscreen(window::Mode),
}

impl RemoteMods {
	fn load() -> Command<Message> {
		Command::perform(SavedState::load(), Message::Loaded)
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SavedState {
	input_value: String,
	filter: Filter,
	tasks: Vec<Task>,
}

#[derive(Debug, Clone)]
enum LoadError {
	File,
	Format,
}

#[derive(Debug, Clone)]
enum SaveError {
	File,
	Write,
	Format,
}
impl SavedState {
	fn path() -> std::path::PathBuf {
		let mut path = if let Some(project_dirs) =
			directories_next::ProjectDirs::from("rs", "Iced", "Todos")
		{
			project_dirs.data_dir().into()
		} else {
			std::env::current_dir().unwrap_or_default()
		};

		path.push("todos.json");

		path
	}

	async fn load() -> Result<SavedState, LoadError> {
		use async_std::prelude::*;

		let mut contents = String::new();

		let mut file = async_std::fs::File::open(Self::path())
			.await
			.map_err(|_| LoadError::File)?;

		file.read_to_string(&mut contents)
			.await
			.map_err(|_| LoadError::File)?;

		serde_json::from_str(&contents).map_err(|_| LoadError::Format)
	}

	async fn save(self) -> Result<(), SaveError> {
		use async_std::prelude::*;

		let json = serde_json::to_string_pretty(&self)
			.map_err(|_| SaveError::Format)?;

		let path = Self::path();

		if let Some(dir) = path.parent() {
			async_std::fs::create_dir_all(dir)
				.await
				.map_err(|_| SaveError::File)?;
		}

		{
			let mut file = async_std::fs::File::create(path)
				.await
				.map_err(|_| SaveError::File)?;

			file.write_all(json.as_bytes())
				.await
				.map_err(|_| SaveError::Write)?;
		}

		// This is a simple way to save at most once every couple seconds
		async_std::task::sleep(std::time::Duration::from_secs(2)).await;

		Ok(())
	}
}