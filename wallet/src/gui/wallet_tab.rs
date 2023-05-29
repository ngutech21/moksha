use iced::{
    widget::{qr_code::State, Column, Container, QRCode, Row, TextInput},
    Element, Length,
};
use iced_aw::TabLabel;

use super::{
    util::{btn, mint_icon, pay_icon, receive_icon},
    Message, Tab,
};

use super::util::h1;
use super::util::Collection;

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
                .spacing(10)
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
                            .push(
                                btn(Some(mint_icon()), "Mint")
                                    .on_press(Message::ShowMintTokensPopup),
                            )
                            .push(
                                btn(Some(receive_icon()), "Receive")
                                    .on_press(Message::ShowReceiveTokensPopup),
                            )
                            .push(
                                btn(Some(pay_icon()), "Pay").on_press(Message::ShowPayInvoicePopup),
                            ),
                    )
                }),
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
