use iced::{Element, Task};

mod mod_entry;
mod mods;

fn main() -> iced::Result {
	iced::application(ModManager::title, ModManager::update, ModManager::view).run_with(ModManager::new)
}

enum ModManager {
	Loading,
	Loaded(State)
}

struct State {
	
}

enum Menu{
	ModConfiguration,
}

#[derive(Debug)]
enum Message {
	
}

impl ModManager {
	fn title(&self) -> String{
		"Mod Manager".to_string()
	}
	fn update(&mut self, message: Message) -> Task<Message>{

	}

	fn view(&self) -> Element<Message>{

	}

	fn new() -> (Self, Task<Message>){
		(Self::Loading,
		 Task::perform(SavedState::load(), Message::Loaded),
		)
	}
}

