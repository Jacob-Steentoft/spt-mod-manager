use iced::alignment::Horizontal::Center;
use iced::widget::{button, row, text, text_input, Text};
use iced::{Element, Font};
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
enum ConfigurationMessage {
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
	pub fn view(&self) -> Element<ConfigurationMessage> {
		match self.state {
			ConfigurationState::Idle => {
				let current = &self.current;
				row!(
					text(&current.url),
					text(&current.version),
					text(&current.version_filter),
					text(&current.github_pattern),
					button(edit_icon())
						.on_press(ConfigurationMessage::Edit)
						.padding(10)
						.width(90)
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
					text_input(&current.github_pattern, &modified.github_pattern)
						.on_input(ConfigurationMessage::GithubPatternChanged),
					button("Save")
						.on_press(ConfigurationMessage::Save)
						.padding(10)
						.width(30),
					button("Cancel")
						.on_press(ConfigurationMessage::Cancel)
						.padding(10)
						.width(30),
					button("Delete")
						.on_press(ConfigurationMessage::Delete)
						.padding(10)
						.width(30),
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

fn text_icon(unicode: char) -> Text<'static> {
	text(unicode.to_string())
		.font(ICONS)
		.width(20)
		.align_x(Center)
}

const ICONS: Font = Font::with_name("Iced-Todos-Icons");

fn edit_icon() -> Text<'static> {
	text_icon('\u{F303}')
}

impl From<ModVersionConfiguration> for ModConfigEntryView {
	fn from(value: ModVersionConfiguration) -> Self {
		Self {
			current: ModConfigEntry {
				url: value.url,
				version: value.version.map_or(String::new(), |x| x.to_string()),
				version_filter: value
					.version_filter
					.map_or(String::new(), |x| x.to_string()),
				github_pattern: value
					.github_pattern
					.map_or(String::new(), |x| x.to_string()),
				github_filter: value.github_filter.map_or(String::new(), |x| x.to_string()),
			},
			modified: Default::default(),
			state: Default::default(),
		}
	}
}
