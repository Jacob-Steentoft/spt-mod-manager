use crate::mod_configurations::{ModConfigurationsMessage, ModConfigurationsView};
use iced::{Element, Task};

mod mod_configurations;
mod mod_entry;

fn main() -> iced::Result {
	iced::application(
		ModManagerView::title,
		ModManagerView::update,
		ModManagerView::view,
	)
	.run()
}

#[derive(Default, Debug)]
struct ModManagerView {
	menu: Menu,
}

#[derive(Debug)]
enum Menu {
	ModConfiguration(ModConfigurationsView),
}

impl Default for Menu {
	fn default() -> Self {
		Self::ModConfiguration(ModConfigurationsView::default())
	}
}

#[derive(Debug)]
enum ModManagerMessage {
	ModConfigurationMessage(ModConfigurationsMessage),
}

impl ModManagerView {
	fn title(&self) -> String {
		"Mod Manager".to_string()
	}
	fn update(&mut self, message: ModManagerMessage) -> Task<ModManagerMessage> {
		match (&mut self.menu, message) {
			(
				Menu::ModConfiguration(ref mut view),
				ModManagerMessage::ModConfigurationMessage(message),
			) => view
				.update(message)
				.map(ModManagerMessage::ModConfigurationMessage),
		}
	}

	fn view(&self) -> Element<ModManagerMessage> {
		match &self.menu {
			Menu::ModConfiguration(mod_cfg) => mod_cfg
				.view()
				.map(ModManagerMessage::ModConfigurationMessage),
		}
	}
}
