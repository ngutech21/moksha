use iced::{
    widget::{button, Column, Container, Text, TextInput},
    Element, Length,
};
use iced_aw::{style::NumberInputStyles, NumberInput, TabLabel};

use crate::wallet;

use super::{Message, Tab};

pub struct WalletTab {
    pub balance: u64,
    pub invoice: String,
    pub mint_token_amount: u64,
    pub wallet: wallet::Wallet,
}

impl WalletTab {
    // async fn mint(wallet: &wallet::Wallet) -> Result<Tokens, CashuWalletError> {
    //     wallet.mint_tokens(100, "lnbc".to_string()).await
    // }

    // pub fn update(&'static mut self, message: WalletMessage) -> Command<Message> {
    //     match message {
    //         WalletMessage::MintPressed => {
    //             let w = &self.wallet;
    //             return Command::perform(Self::mint(w), Message::Something);
    //         }
    //         WalletMessage::InvoiceTextChanged(invoice) => self.invoice = invoice,
    //         WalletMessage::MintTokenAmountChanged(amt) => self.mint_token_amount = amt,
    //     }
    //     Command::none()
    // }

    pub fn update(&mut self, message: WalletMessage) {
        match message {
            WalletMessage::MintPressed => {}
            WalletMessage::InvoiceTextChanged(invoice) => self.invoice = invoice,
            WalletMessage::MintTokenAmountChanged(amt) => self.mint_token_amount = amt,
        }
    }
}

#[derive(Debug, Clone)]
pub enum WalletMessage {
    MintPressed,
    InvoiceTextChanged(String),
    MintTokenAmountChanged(u64),
}

impl Tab for WalletTab {
    type Message = Message;

    fn title(&self) -> String {
        String::from("Wallet")
    }

    fn tab_label(&self) -> TabLabel {
        TabLabel::IconText('\u{E800}', self.title())
    }

    fn content(&self) -> Element<'_, Self::Message> {
        let content: Element<'_, WalletMessage> = Container::new(
            Column::new()
                .push(Text::new(format!("Balance {} (sats)", self.balance)))
                .push(
                    TextInput::new("lnbc...", &self.invoice)
                        .on_input(WalletMessage::InvoiceTextChanged)
                        .padding(15)
                        .size(30),
                )
                .push(
                    NumberInput::new(
                        self.mint_token_amount,
                        1_000,
                        WalletMessage::MintTokenAmountChanged,
                    )
                    .style(NumberInputStyles::Default)
                    .step(100)
                    .padding(15.0)
                    .size(30.0),
                )
                .push(button("Mint").on_press(WalletMessage::MintPressed)),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y()
        .into();
        content.map(Message::Wallet)
    }
}
