use std::time::Duration;

use cashurs_wallet::client::HttpClient;
use cashurs_wallet::gui::{settings_tab, wallet_tab, Message};
use cashurs_wallet::localstore::RocksDBLocalStore;
use cashurs_wallet::wallet::{self, Wallet};
use dotenvy::dotenv;
use iced::widget::qr_code::State;
use iced::{theme, Color, Settings};
use iced::{Application, Command, Font, Theme};
use iced_aw::tabs::TabBarStyles;
use iced_aw::Tabs;

use cashurs_wallet::gui::Tab;

use cashurs_wallet::client::Client;
use tokio::time::{sleep_until, Instant};

const ICON_FONT: Font = iced::Font::External {
    name: "Icons",
    bytes: include_bytes!("../../fonts/boxicons.ttf"),
};

fn read_env(variable: &str) -> String {
    dotenv().expect(".env file not found");
    std::env::var(variable).expect("MINT_URL not found")
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mint_url = read_env("WALLET_MINT_URL");
    let client = cashurs_wallet::client::HttpClient::new(mint_url.clone());
    let keys = client.get_mint_keys().await?;
    let keysets = client.get_mint_keysets().await?;
    let localstore = Box::new(RocksDBLocalStore::new(read_env("WALLET_DB_PATH")));
    let wallet = wallet::Wallet::new(Box::new(client), keys, keysets, localstore, mint_url);

    let mut settings = Settings::with_flags(wallet);
    settings.antialiasing = true;
    settings.window.size = (800, 600);
    settings.window.resizable = true;

    MainFrame::run(settings)?;
    Ok(())
}

pub struct MainFrame {
    active_tab: usize,
    settings_tab: settings_tab::SettingsTab,
    wallet_tab: wallet_tab::WalletTab,

    wallet: wallet::Wallet,
    client: HttpClient,
}

impl Application for MainFrame {
    type Message = Message;
    type Theme = Theme;
    type Executor = iced::executor::Default;
    type Flags = Wallet;

    fn new(wallet: Wallet) -> (MainFrame, Command<Message>) {
        let mint_url = read_env("WALLET_MINT_URL");
        let client = cashurs_wallet::client::HttpClient::new(mint_url);

        let balance = wallet.get_balance().expect("msg");
        (
            MainFrame {
                active_tab: 0,
                settings_tab: settings_tab::SettingsTab::new(),
                wallet_tab: wallet_tab::WalletTab {
                    invoice_hash: None,
                    invoice: None,
                    mint_token_amount: None,
                    balance,
                    qr_code: None,
                },
                wallet,
                client,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Cashu-rs Wallet")
    }

    fn update(&mut self, message: Self::Message) -> Command<Message> {
        match message {
            Message::TokensMinted(balance) => {
                println!("new balance: {:?}", balance);
                self.wallet_tab.balance = balance.unwrap_or(0);
                self.wallet_tab.qr_code = None;
                self.wallet_tab.invoice = None;
                self.wallet_tab.invoice_hash = None;
                self.wallet_tab.mint_token_amount = None;
                Command::none()
            }
            Message::TabSelected(index) => {
                self.active_tab = index;
                Command::none()
            }
            Message::Settings(message) => {
                self.settings_tab.update(message);
                Command::none()
            }
            Message::CreateInvoicePressed => {
                let amt = match self.wallet_tab.mint_token_amount {
                    Some(amt) => amt,
                    None => return Command::none(),
                };

                let cl = self.client.clone();
                Command::perform(
                    async move {
                        cl.get_mint_payment_request(amt)
                            .await
                            .map_err(|err| err.to_string())
                    },
                    Message::PaymentRequestReceived,
                )
            }
            Message::InvoiceTextChanged(invoice) => {
                self.wallet_tab.invoice = Some(invoice);
                Command::none()
            }
            Message::MintTokenAmountChanged(amt) => {
                self.wallet_tab.mint_token_amount = Some(amt);
                Command::none()
            }

            Message::PaymentRequestReceived(pr) => {
                match pr {
                    Ok(pr) => {
                        self.wallet_tab.invoice = Some(pr.pr.clone());
                        self.wallet_tab.invoice_hash = Some(pr.hash.clone());
                        self.wallet_tab.qr_code = State::new(&pr.pr).ok();

                        let hash = pr.hash;
                        let amt = self.wallet_tab.mint_token_amount;
                        if amt.is_none() {
                            return Command::none();
                        }

                        let wallet = self.wallet.clone();

                        return Command::perform(
                            async move {
                                loop {
                                    sleep_until(Instant::now() + Duration::from_millis(1_000))
                                        .await;
                                    let mint_result =
                                        wallet.mint_tokens(amt.unwrap(), hash.clone()).await;

                                    match mint_result {
                                        Ok(_) => {
                                            break;
                                        }
                                        Err(cashurs_wallet::error::CashuWalletError::InvoiceNotPaidYet(_, _)) => {
                                            continue;
                                        }
                                        Err(e) => {
                                            println!("Error: {:?}", e);
                                            break;
                                        }
                                    }
                                }
                                wallet.get_balance().map_err(|err| err.to_string())
                            },
                            Message::TokensMinted,
                        );
                    }
                    Err(e) => {
                        println!("Error: {:?}", e);
                    }
                }

                Command::none()
            }
        }
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
        Theme::custom(theme::Palette {
            background: Color::from_rgb8(37, 37, 37),
            text: Color::BLACK,
            primary: Color::from_rgb8(94, 124, 226),
            success: Color::from_rgb8(8, 102, 79),
            danger: Color::from_rgb8(195, 66, 63),
        })
    }
}
