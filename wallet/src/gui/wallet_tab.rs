use std::borrow::Cow;

use iced::{
    widget::{button, qr_code::State, Column, Container, QRCode, Row, TextInput},
    Element, Length, Theme,
};
use iced_aw::{style::NumberInputStyles, NumberInput, TabLabel};

use super::{Message, Tab};

pub struct WalletTab {
    pub balance: u64,
    pub invoice: Option<String>,
    pub invoice_hash: Option<String>,
    pub qr_code: Option<State>,
    pub mint_token_amount: Option<u64>,
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

pub fn text<'a>(content: impl Into<Cow<'a, str>>) -> iced::widget::Text<'a, iced::Renderer<Theme>> {
    p1_regular(content)
}

pub fn h1<'a>(content: impl Into<Cow<'a, str>>) -> iced::widget::Text<'a, iced::Renderer<Theme>> {
    iced::widget::Text::new(content).size(48)
}

pub fn p1_regular<'a>(
    content: impl Into<Cow<'a, str>>,
) -> iced::widget::Text<'a, iced::Renderer<Theme>> {
    iced::widget::Text::new(content).size(20)
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
        TabLabel::IconText('\u{EC78}', self.title())
    }

    fn content(&self) -> Element<'_, Self::Message> {
        let content: Element<'_, Message> = Container::new(
            Column::new()
                .push(h1(format!("Balance {} (sats)", self.balance)))
                .push_maybe(self.qr_code.as_ref().map(QRCode::new))
                .push(
                    TextInput::new("", &self.invoice.clone().unwrap_or_default())
                        .on_input(Message::InvoiceTextChanged),
                )
                .push(
                    Row::new().push(text("Amount (sats)")).push(
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
                .push(
                    Row::new()
                        .align_items(iced::Alignment::Center)
                        .spacing(10)
                        .push(button("Create Invoice").on_press(Message::CreateInvoicePressed))
                        .push(
                            Column::new()
                                .push(button("Mint Tokens").on_press(Message::MintPressed)),
                        ),
                ),
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
