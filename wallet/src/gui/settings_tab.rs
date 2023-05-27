use iced::{
    widget::{button, Column, Container, TextInput},
    Element, Length,
};
use iced_aw::TabLabel;

use super::{Message, Tab};

#[derive(Debug)]
pub struct SettingsTab {
    mint_url: String,
}

impl SettingsTab {
    pub fn new() -> Self {
        Self {
            mint_url: String::from("https://127.0.0.1:3338"),
        }
    }

    pub fn update(&mut self, message: SettingsMessage) {
        match message {
            SettingsMessage::MintTextChanged(value) => self.mint_url = value,
            SettingsMessage::ChangeMintPressed => {
                println!("change mint pressed")
            }
        }
        println!("event {:?}", &self);
    }
}

impl Default for SettingsTab {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub enum SettingsMessage {
    ChangeMintPressed,
    MintTextChanged(String),
}

impl Tab for SettingsTab {
    type Message = Message;

    fn title(&self) -> String {
        String::from("Settings")
    }

    fn tab_label(&self) -> TabLabel {
        TabLabel::IconText('\u{E5E9}', self.title())
    }

    fn content(&self) -> Element<'_, Self::Message> {
        let content: Element<'_, SettingsMessage> = Container::new(
            Column::new()
                .push(
                    TextInput::new("placeholder", &self.mint_url)
                        .on_input(SettingsMessage::MintTextChanged)
                        .padding(15)
                        .size(30),
                )
                .push(
                    Column::new()
                        .push(button("Change Mint").on_press(SettingsMessage::ChangeMintPressed)),
                ),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y()
        .into();
        let st = Message::Settings;
        content.map(st)
    }
}
