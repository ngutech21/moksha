use std::collections::HashMap;

use cashurs_core::model::Keysets;
use cashurs_wallet::gui::{settings_tab, wallet_tab, Message};
use cashurs_wallet::localstore::RocksDBLocalStore;
use cashurs_wallet::wallet;
use dotenvy::dotenv;
use iced::Settings;
use iced::{Application, Command, Font, Theme};
use iced_aw::tabs::TabBarStyles;
use iced_aw::Tabs;

use cashurs_wallet::gui::Tab;

const ICON_FONT: Font = iced::Font::External {
    name: "Icons",
    bytes: include_bytes!("../../fonts/icons.ttf"),
};

fn read_env(variable: &str) -> String {
    dotenv().expect(".env file not found");
    std::env::var(variable).expect("MINT_URL not found")
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    MainFrame::run(Settings::default())?;
    Ok(())
}

pub struct MainFrame {
    active_tab: usize,
    settings_tab: settings_tab::SettingsTab,
    wallet_tab: wallet_tab::WalletTab,
}

impl Application for MainFrame {
    type Message = Message;
    type Theme = Theme;
    type Executor = iced::executor::Default;
    type Flags = ();

    fn new(_flags: ()) -> (MainFrame, Command<Message>) {
        let mint_url = read_env("WALLET_MINT_URL");
        let client = cashurs_wallet::client::HttpClient::new(mint_url.clone());

        //let keys = client.get_mint_keys().await.expect("msg");
        let keys = HashMap::new();
        let keysets = Keysets {
            keysets: Vec::new(),
        };

        let localstore = Box::new(RocksDBLocalStore::new(read_env("WALLET_DB_PATH")));

        let wallet = wallet::Wallet::new(Box::new(client), keys, keysets, localstore, mint_url);

        let balance = wallet.get_balance().expect("msg");

        (
            MainFrame {
                active_tab: 0,
                settings_tab: settings_tab::SettingsTab::new(),
                wallet_tab: wallet_tab::WalletTab {
                    invoice: "".to_string(),
                    mint_token_amount: 0,
                    balance,
                    wallet,
                },
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Cashu-rs Wallet")
    }

    fn update(&mut self, message: Self::Message) -> Command<Message> {
        match message {
            Message::TabSelected(index) => self.active_tab = index,
            Message::Settings(message) => self.settings_tab.update(message),
            Message::Wallet(message) => self.wallet_tab.update(message),
            Message::Something(result) => {
                match result {
                    Ok(_tokens) => {
                        //self.wallet_tab.balance = tokens.amount;
                    }
                    Err(e) => {
                        println!("Error: {}", e);
                    }
                }
            }
        }
        Command::none()
    }

    fn view(&self) -> iced::Element<Self::Message> {
        Tabs::new(self.active_tab, Message::TabSelected)
            .push(self.wallet_tab.tab_label(), self.wallet_tab.view())
            .push(self.settings_tab.tab_label(), self.settings_tab.view())
            .tab_bar_style(TabBarStyles::Blue)
            .icon_font(ICON_FONT)
            .tab_bar_position(iced_aw::TabBarPosition::Top)
            .into()
    }

    fn theme(&self) -> iced::Theme {
        Theme::Dark
    }
}
