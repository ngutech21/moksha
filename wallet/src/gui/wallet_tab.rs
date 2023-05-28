use iced::{
    widget::{button, qr_code::State, Column, Container, QRCode, Row, TextInput},
    Element, Length,
};
use iced_aw::{style::NumberInputStyles, NumberInput, TabLabel};

use super::{Message, Tab};

use super::util::Collection;
use super::util::{h1, text};

pub struct WalletTab {
    pub balance: u64,
    pub invoice: Option<String>,
    pub invoice_hash: Option<String>,
    pub qr_code: Option<State>,
    pub mint_token_amount: Option<u64>,
}

impl Tab for WalletTab {
    type Message = Message;

    fn title(&self) -> String {
        String::from("Wallet")
    }

    fn tab_label(&self) -> TabLabel {
        TabLabel::IconText('\u{EC78}', self.title())
    }

    fn content(&self) -> Element<'_, Self::Message> {
        let content: Element<'_, Message> = Container::new(
            Column::new()
                .push(h1(format!("{} (sats)", self.balance)))
                .spacing(100)
                .push_maybe(self.qr_code.as_ref().map(QRCode::new))
                .push_maybe(self.invoice.as_ref().map(|_| {
                    TextInput::new("", &self.invoice.clone().unwrap_or_default())
                        .on_input(Message::InvoiceTextChanged)
                }))
                .push_maybe(if self.qr_code.is_some() {
                    None
                } else {
                    Some(
                        Row::new()
                            .align_items(iced::Alignment::Center)
                            .spacing(10)
                            .push(button("Mint Tokens").on_press(Message::CreateInvoicePressed))
                            .push(text("Amount (sats)"))
                            .push(
                                NumberInput::new(
                                    self.mint_token_amount.unwrap_or_default(),
                                    1_000,
                                    Message::MintTokenAmountChanged,
                                )
                                .min(1)
                                .style(NumberInputStyles::Default)
                                .step(100),
                            ),
                    )
                })
                .push(button("Receive Tokens").on_press(Message::ShowReceiveTokensPopup)),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y()
        .into();
        //content.explain(Color::WHITE)
        content
    }
}
