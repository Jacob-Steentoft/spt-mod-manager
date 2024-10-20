use iced::{window, Element, Fill, Task};
use iced::alignment::Horizontal::Center;
use iced::widget::{container, text};
use sptmm_lib::configuration_access::ModVersionConfiguration;
use sptmm_lib::spt_access::SptAccess;
use sptmm_lib::time_access::Time;

#[derive(Default, Debug)]
enum ModConfigurations {
	#[default]
	Loading,
	Loaded(State),
}

#[derive(Debug)]
struct State {
	remote_client: SptAccess<Time>,
	remote_mods: Vec<ModVersionConfiguration>,
	dirty: bool,
	saving: bool,
}

#[derive(Debug, Clone)]
enum Message {
	Loaded(Result<SavedState, LoadError>),
	Saved(Result<(), SaveError>),
	InputChanged(String),
	CreateCfgVersion,
	TabPressed { shift: bool },
	ToggleFullscreen(window::Mode),
}


impl ModConfigurations {
	fn load() -> Task<Message> {
		Task::perform(SavedState::load(), Message::Loaded)
	}

	fn update(&self, message: Message) -> Task<Message> {
		match self {
			ModConfigurations::Loading => match message {
				Message::Loaded(state) => {}
				Message::Saved(_) => {}
				Message::InputChanged(_) => {}
				Message::CreateCfgVersion => {}
				Message::TabPressed { .. } => {}
				Message::ToggleFullscreen(_) => {}
			},
			ModConfigurations::Loaded(state) => {}
		}
	}

	fn view(&self) -> Element<Message> {
		match self {
			ModConfigurations::Loading => container(
				text("Loading...")
					.width(Fill)
					.horizontal_alignment(Center)
					.size(50),
			)
				.center_y()
				.into(),
			ModConfigurations::Loaded(State { remote_mods, .. }) => keyed_column(remote_mods.iter().map()),
		}
	}
}

#[derive(Debug, Clone)]
struct SavedState {
	cfg_access: ConfigurationAccess,
	cfg: ModConfiguration,
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
	async fn load() -> Result<Self, LoadError> {
		let cfg_access = ConfigurationAccess::init("./").await.unwrap();
		let cfg = cfg_access.read_remote_mods().await.unwrap();
		let state = Self { cfg, cfg_access };
		Ok(state)
	}

	async fn save(&self) -> Result<(), SaveError> {
		self.cfg_access.write_remote_mods(&self.cfg).await.unwrap();
		Ok(())
	}
}