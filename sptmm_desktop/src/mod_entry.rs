use iced::alignment::Horizontal::Center;
use iced::widget::{button, horizontal_space, row, text, text_input, Text};
use iced::{Background, Element, Font};
use sptmm_lib::configuration_access::ModVersionConfiguration;

// https://github.com/iced-rs/iced/tree/master/examples/todos

#[derive(Debug, Clone, Default)]
struct ModConfigEntry {
	url: String,
	version: String,
	version_filter: String,
	github_filter: String,
	github_pattern: String,
}

#[derive(Debug, Clone)]
pub struct ModConfigEntryView {
	current: ModConfigEntry,
	modified: ModConfigEntry,
	state: ConfigurationState,
}

#[derive(Debug, Clone, Default)]
enum ConfigurationState {
	#[default]
	Idle,
	Editing,
}

#[derive(Debug, Clone)]
pub(crate) enum ConfigurationMessage {
	UrlChanged(String),
	VersionChanged(String),
	VersionFilterChanged(String),
	GithubFilterChanged(String),
	GithubPatternChanged(String),
	Edit,
	Save,
	Cancel,
	Delete,
}

impl ModConfigEntryView {
	pub fn new(mod_entry: &ModVersionConfiguration) -> Self {
		mod_entry.clone().into()
	}

	pub fn view(&self) -> Element<ConfigurationMessage> {
		match self.state {
			ConfigurationState::Idle => {
				let current = &self.current;
				row!(
					text_input("", &current.url),
					text_input("", &current.version),
					text_input("", &current.version_filter),
					text_input("", &current.github_filter),
					text_input("", &current.github_pattern),
					horizontal_space(),
					button(text("Edit").center()).on_press(ConfigurationMessage::Edit)
				)
				.into()
			}
			ConfigurationState::Editing => {
				let current = &self.current;
				let modified = &self.modified;
				row!(
					text_input(&current.url, &modified.url)
						.on_input(ConfigurationMessage::UrlChanged),
					text_input(&current.version, &modified.version)
						.on_input(ConfigurationMessage::VersionChanged),
					text_input(&current.version_filter, &modified.version_filter)
						.on_input(ConfigurationMessage::VersionFilterChanged),
					text_input(&current.github_filter, &modified.github_filter)
						.on_input(ConfigurationMessage::GithubFilterChanged),
					text_input(&current.github_pattern, &modified.github_pattern)
						.on_input(ConfigurationMessage::GithubPatternChanged),
					horizontal_space(),
					button("Save").on_press(ConfigurationMessage::Save),
					button("Cancel").on_press(ConfigurationMessage::Cancel),
					button("Delete").on_press(ConfigurationMessage::Delete),
				)
				.into()
			}
		}
	}

	pub fn update(&mut self, message: ConfigurationMessage) {
		match message {
			ConfigurationMessage::Edit => {
				self.state = ConfigurationState::Editing;
				self.modified = self.current.clone();
			}
			ConfigurationMessage::Save => {
				self.state = ConfigurationState::Editing;
				self.current = self.modified.clone();
			}
			ConfigurationMessage::Cancel => {
				self.state = ConfigurationState::Idle;
				self.modified = Default::default();
			}
			ConfigurationMessage::Delete => {}
			ConfigurationMessage::UrlChanged(url) => {
				self.modified.url = url;
			}
			ConfigurationMessage::VersionChanged(version) => {
				self.modified.version = version;
			}
			ConfigurationMessage::VersionFilterChanged(version_filter) => {
				self.modified.version_filter = version_filter;
			}
			ConfigurationMessage::GithubFilterChanged(github_filter) => {
				self.modified.github_pattern = github_filter;
			}
			ConfigurationMessage::GithubPatternChanged(github_pattern) => {
				self.modified.github_pattern = github_pattern;
			}
		}
	}
}

impl From<ModVersionConfiguration> for ModConfigEntryView {
	fn from(value: ModVersionConfiguration) -> Self {
		Self {
			current: ModConfigEntry {
				url: value.url,
				version: value.version.map_or(String::new(), |x| x.to_string()),
				version_filter: value.version_filter.unwrap_or_default(),
				github_pattern: value.github_pattern.unwrap_or_default(),
				github_filter: value.github_filter.unwrap_or_default(),
			},
			modified: Default::default(),
			state: Default::default(),
		}
	}
}
