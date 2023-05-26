use iced::{
    widget::{button, qr_code::State, Column, Container, QRCode, Row, Text},
    Element, Length,
};
use iced_aw::{style::NumberInputStyles, NumberInput, TabLabel};

use super::{Message, Tab};

pub struct WalletTab {
    pub balance: u64,
    pub invoice: String,
    pub qr_code: Option<State>,
    pub mint_token_amount: u64,
}

pub trait Collection<'a, Message>: Sized {
    fn push(self, element: impl Into<Element<'a, Message>>) -> Self;

    fn push_maybe(self, element: Option<impl Into<Element<'a, Message>>>) -> Self {
        match element {
            Some(element) => self.push(element),
            None => self,
        }
    }
}

impl<'a, Message> Collection<'a, Message> for Column<'a, Message> {
    fn push(self, element: impl Into<Element<'a, Message>>) -> Self {
        Self::push(self, element)
    }
}

impl<'a, Message> Collection<'a, Message> for Row<'a, Message> {
    fn push(self, element: impl Into<Element<'a, Message>>) -> Self {
        Self::push(self, element)
    }
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
        let content: Element<'_, Message> = Container::new(
            Column::new()
                .push(Text::new(format!("Balance {} (sats)", self.balance)).size(18))
                .push_maybe(self.qr_code.as_ref().map(QRCode::new))
                .push(Text::new(&self.invoice).size(12))
                .push(
                    NumberInput::new(
                        self.mint_token_amount,
                        1_000,
                        Message::MintTokenAmountChanged,
                    )
                    .style(NumberInputStyles::Default)
                    .step(100)
                    .padding(15.0)
                    .size(30.0),
                )
                .push(button("Create Invoice").on_press(Message::CreateInvoicePressed))
                .push(button("Mint").on_press(Message::MintPressed)),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y()
        .into();
        content
    }
}
