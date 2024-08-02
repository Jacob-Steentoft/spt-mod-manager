use iced::Element;
use iced::widget::row;
use sptmm_lib::configuration_access::ModVersionConfiguration;

struct ModVersionConfigurationView {
	base: ModVersionConfiguration,
	state: ConfigurationState,
}

#[derive(Debug, Clone, Default)]
enum ConfigurationState{
	#[default]
	Idle,
	Editing
}

#[derive(Debug, Clone)]
enum ConfigurationMessage{
	Completed(bool),
	Edit,
	FinishEdition,
	Delete,
}

impl ModVersionConfigurationView {
	fn view(&self) -> Element<ConfigurationMessage>{
		match self.state {
			ConfigurationState::Idle => {
				let configuration = &self.base;
				row!(configuration.)
			}
			ConfigurationState::Editing => {}
		}
	}
}

impl From<ModVersionConfiguration> for ModVersionConfigurationView{
	fn from(value: ModVersionConfiguration) -> Self {
		Self{
			base: value,
			state: Default::default()
		}
	}
}

