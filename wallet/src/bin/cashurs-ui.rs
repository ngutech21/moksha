use std::time::Duration;

use cashurs_core::model::Tokens;
use cashurs_wallet::client::HttpClient;
use cashurs_wallet::gui::Tab;
use cashurs_wallet::gui::{settings_tab, wallet_tab, Message};
use cashurs_wallet::localstore::RocksDBLocalStore;
use cashurs_wallet::wallet::{self, Wallet};
use dotenvy::dotenv;
use iced::alignment::Horizontal;
use iced::widget::qr_code::State;
use iced::widget::{button, text_input, Row};
use iced::{Application, Command, Font, Theme};
use iced::{Length, Settings};
use iced_aw::style::NumberInputStyles;
use iced_aw::tabs::TabBarStyles;
use iced_aw::{Card, Modal, NumberInput, Tabs};

use cashurs_wallet::client::Client;
use tokio::time::{sleep_until, Instant};

use cashurs_wallet::gui::util::text;
use cashurs_wallet::gui::util::Collection;

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
    show_receive_tokens_modal: bool,
    show_mint_tokens_modal: bool,
    show_pay_invoice_modal: bool,
    receive_token: String,
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
                    pay_invoice: None,
                },
                wallet,
                client,
                show_receive_tokens_modal: false,
                show_mint_tokens_modal: false,
                show_pay_invoice_modal: false,
                receive_token: String::new(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Cashu-rs Wallet")
    }

    fn update(&mut self, message: Self::Message) -> Command<Message> {
        match message {
            Message::PayInvoiceChanged(invoice) => {
                self.wallet_tab.pay_invoice = Some(invoice);
                Command::none()
            }
            Message::PayInvoicePressed => {
                self.show_pay_invoice_modal = false;
                let wallet = self.wallet.clone();
                let invoice = self.wallet_tab.pay_invoice.clone().unwrap();
                self.wallet_tab.pay_invoice = None;

                Command::perform(
                    async move {
                        wallet.pay_invoice(invoice).await.unwrap(); // FIXME handle error
                        wallet.get_balance().map_err(|err| err.to_string())
                    },
                    Message::TokenBalanceChanged,
                )
            }
            Message::ShowPayInvoicePopup => {
                self.show_pay_invoice_modal = true;
                Command::none()
            }
            Message::HideInvoicePopup => {
                self.show_pay_invoice_modal = false;
                Command::none()
            }
            Message::ShowMintTokensPopup => {
                self.show_mint_tokens_modal = true;
                Command::none()
            }
            Message::HideMintTokensPopup => {
                self.show_mint_tokens_modal = false;
                Command::none()
            }
            Message::ImportTokensPressed => {
                println!("import tokens pressed");

                let token = self.receive_token.clone();
                let tokens = Tokens::deserialize(token).unwrap();
                let total_amount = tokens.total_amount();
                self.show_receive_tokens_modal = false;

                let wallet = self.wallet.clone();
                Command::perform(
                    async move {
                        let (_, redeemed_tokens) =
                            wallet.split_tokens(tokens, total_amount).await.unwrap();
                        let _ = wallet.localstore().add_tokens(redeemed_tokens); // FIXME error handling
                        wallet.get_balance().map_err(|err| err.to_string())
                    },
                    Message::TokenBalanceChanged,
                )
            }
            Message::ReceiveTokenChanged(token) => {
                self.receive_token = token;
                println!("token changed: {:?}", &self.receive_token);
                Command::none()
            }
            Message::ShowReceiveTokensPopup => {
                print!("Receive tokens pressed");
                self.show_receive_tokens_modal = true;
                Command::none()
            }
            Message::HideReceiveTokensPopup => {
                print!("Hide tokens pressed");
                self.show_receive_tokens_modal = false;
                Command::none()
            }
            Message::TokenBalanceChanged(balance) => {
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
                self.show_mint_tokens_modal = false;
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
                            Message::TokenBalanceChanged,
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
        let content = Tabs::new(self.active_tab, Message::TabSelected)
            .push(self.wallet_tab.tab_label(), self.wallet_tab.view())
            .push(self.settings_tab.tab_label(), self.settings_tab.view())
            .tab_bar_style(TabBarStyles::Blue)
            .icon_font(ICON_FONT)
            .tab_bar_position(iced_aw::TabBarPosition::Top);

        if self.show_receive_tokens_modal {
            Modal::new(self.show_receive_tokens_modal, content, || {
                Card::new(
                    text("Receive Tokens"),
                    text_input("Token", &self.receive_token).on_input(Message::ReceiveTokenChanged),
                )
                .foot(
                    Row::new()
                        .spacing(10)
                        .padding(5)
                        .width(Length::Fill)
                        .push(
                            button(text("Cancel").horizontal_alignment(Horizontal::Center))
                                .width(Length::Fill)
                                .on_press(Message::HideReceiveTokensPopup),
                        )
                        .push(
                            button(text("Import Tokens").horizontal_alignment(Horizontal::Center))
                                .width(Length::Fill)
                                .on_press(Message::ImportTokensPressed),
                        ),
                )
                .max_width(500.0)
                .on_close(Message::HideReceiveTokensPopup)
                .into()
            })
            .backdrop(Message::HideReceiveTokensPopup)
            .on_esc(Message::HideReceiveTokensPopup)
            .into()
        } else if self.show_mint_tokens_modal {
            Modal::new(self.show_mint_tokens_modal, content, || {
                Card::new(
                    text("Enter amount in sats"),
                    NumberInput::new(
                        self.wallet_tab.mint_token_amount.unwrap_or_default(),
                        1_000,
                        Message::MintTokenAmountChanged,
                    )
                    .min(1)
                    .style(NumberInputStyles::Default)
                    .step(100),
                )
                .foot(
                    Row::new()
                        .spacing(10)
                        .padding(5)
                        .width(Length::Fill)
                        .push(
                            button(text("Cancel").horizontal_alignment(Horizontal::Center))
                                .width(Length::Fill)
                                .on_press(Message::HideMintTokensPopup),
                        )
                        .push_maybe(
                            if self.wallet_tab.mint_token_amount.is_some()
                                && self.wallet_tab.mint_token_amount.unwrap() > 0
                            {
                                Some(
                                    button(
                                        text("Create invoice")
                                            .horizontal_alignment(Horizontal::Center),
                                    )
                                    .width(Length::Fill)
                                    .on_press(Message::CreateInvoicePressed),
                                )
                            } else {
                                None
                            },
                        ),
                )
                .max_width(500.0)
                .on_close(Message::HideMintTokensPopup)
                .into()
            })
            .backdrop(Message::HideMintTokensPopup)
            .on_esc(Message::HideMintTokensPopup)
            .into()
        } else if self.show_pay_invoice_modal {
            Modal::new(self.show_pay_invoice_modal, content, || {
                Card::new(
                    text("Pay invoice"),
                    text_input(
                        "Invoice",
                        &self.wallet_tab.pay_invoice.clone().unwrap_or_default(),
                    )
                    .on_input(Message::PayInvoiceChanged),
                )
                .foot(
                    Row::new()
                        .spacing(10)
                        .padding(5)
                        .width(Length::Fill)
                        .push(
                            button(text("Cancel").horizontal_alignment(Horizontal::Center))
                                .width(Length::Fill)
                                .on_press(Message::HideInvoicePopup),
                        )
                        .push_maybe(if self.wallet_tab.pay_invoice.is_some() {
                            Some(
                                button(
                                    text("Pay invoice").horizontal_alignment(Horizontal::Center),
                                )
                                .width(Length::Fill)
                                .on_press(Message::PayInvoicePressed),
                            )
                        } else {
                            None
                        }),
                )
                .max_width(500.0)
                .on_close(Message::HideInvoicePopup)
                .into()
            })
            .backdrop(Message::HideInvoicePopup)
            .on_esc(Message::HideInvoicePopup)
            .into()
        } else {
            content.into()
        }
    }

    fn theme(&self) -> iced::Theme {
        // Theme::custom(theme::Palette {
        //     background: Color::from_rgb8(37, 37, 37),
        //     text: Color::BLACK,
        //     primary: Color::from_rgb8(94, 124, 226),
        //     success: Color::from_rgb8(8, 102, 79),
        //     danger: Color::from_rgb8(195, 66, 63),
        // })
        Theme::Dark
    }
}
