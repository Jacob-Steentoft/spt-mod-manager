use crate::mod_entry::{ConfigurationMessage, ModConfigEntryView};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{button, container, keyed_column, scrollable, text};
use iced::{Element, Fill, Task};
use sptmm_lib::configuration_access::{ConfigurationAccess, ModConfiguration};
use sptmm_lib::path_access::PathAccess;

#[derive(Default, Debug)]
enum ModConfigurationsState {
	#[default]
	Load,
	Loading,
	Loaded,
}

#[derive(Default, Debug)]
pub struct ModConfigurationsView {
	remote_mods: Vec<ModConfigEntryView>,
	state: ModConfigurationsState,
}

#[derive(Debug, Clone)]
pub(crate) enum ModConfigurationsMessage {
	ModVersionMessage(usize, ConfigurationMessage),
	Load,
	Loaded(Result<SavedState, LoadError>),
}

impl ModConfigurationsView {
	pub fn update(&mut self, message: ModConfigurationsMessage) -> Task<ModConfigurationsMessage> {
		match (&mut self.state, message) {
			(ModConfigurationsState::Loading, ModConfigurationsMessage::Loaded(state)) => {
				self.remote_mods = state
					.unwrap()
					.cfg
					.mods
					.iter()
					.map(ModConfigEntryView::new)
					.collect();
				self.state = ModConfigurationsState::Loaded;
				Task::none()
			}
			(
				ModConfigurationsState::Loaded,
				ModConfigurationsMessage::ModVersionMessage(index, config_message),
			) => {
				if let Some(mod_config) = self.remote_mods.get_mut(index) {
					mod_config.update(config_message)
				}
				Task::none()
			}
			(_, ModConfigurationsMessage::Load) => {
				self.state = ModConfigurationsState::Loading;
				Task::perform(SavedState::load(), ModConfigurationsMessage::Loaded)
			}
			_ => Task::none(),
		}
	}

	pub fn view(&self) -> Element<ModConfigurationsMessage> {
		match &self.state {
			ModConfigurationsState::Load => button("Load")
				.on_press(ModConfigurationsMessage::Load)
				.into(),
			ModConfigurationsState::Loading => container(
				text("Loading...")
					.width(Fill)
					.align_x(Horizontal::Center)
					.size(50),
			)
			.align_y(Vertical::Center)
			.into(),
			ModConfigurationsState::Loaded => {
				let map =
					self.remote_mods
						.iter()
						.map(|x| x.view())
						.enumerate()
						.map(|(index, view)| {
							(
								index,
								view.map(move |text| {
									ModConfigurationsMessage::ModVersionMessage(index, text)
								}),
							)
						});
				container(scrollable(keyed_column(map).padding([24,10]))).into()
			}
		}
	}
}

#[derive(Debug, Clone)]
pub(crate) struct SavedState {
	cfg_access: ConfigurationAccess,
	cfg: ModConfiguration,
}

#[derive(Debug, Clone)]
pub(crate) enum LoadError {
	File,
	Format,
	EftNotFound,
}

#[derive(Debug, Clone)]
enum SaveError {
	File,
	Write,
	Format,
}

impl SavedState {
	async fn load() -> Result<Self, LoadError> {
		let access = PathAccess::new("C:\\SPT3").map_err(|_| LoadError::EftNotFound)?;
		let cfg_access = ConfigurationAccess::init(&access).await.unwrap();
		let cfg = cfg_access.read_remote_mods().await.unwrap();
		let state = Self { cfg, cfg_access };
		Ok(state)
	}

	async fn save(&self) -> Result<(), SaveError> {
		self.cfg_access.write_remote_mods(&self.cfg).await.unwrap();
		Ok(())
	}
}
