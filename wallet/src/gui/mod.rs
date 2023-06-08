use cashurs_core::model::PaymentRequest;
use iced::{
    alignment::{Horizontal, Vertical},
    widget::{Column, Container},
    Element, Length,
};
use iced_aw::TabLabel;

use self::settings_tab::SettingsMessage;

pub mod settings_tab;
pub mod wallet_tab;

pub mod util;

pub mod components;
pub mod toast;

#[derive(Debug, Clone)]
pub enum Message {
    TabSelected(usize),
    Settings(SettingsMessage),
    InvoiceTextChanged(String),
    MintTokenAmountChanged(u64),
    CreateInvoicePressed,
    PaymentRequestReceived(Result<PaymentRequest, String>),
    TokenBalanceChanged(Result<u64, String>),
    ShowReceiveTokensPopup,
    HideReceiveTokensPopup,
    ShowMintTokensPopup,
    HideMintTokensPopup,
    ShowPayInvoicePopup,
    HideInvoicePopup,
    PayInvoiceChanged(String),
    PayInvoicePressed,
    ImportTokensPressed,
    ReceiveTokenChanged(String),
}

pub trait Tab {
    type Message;

    fn title(&self) -> String;

    fn tab_label(&self) -> TabLabel;

    fn view(&self) -> Element<'_, Self::Message> {
        let column = Column::new().spacing(20).push(self.content());

        Container::new(column)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .padding(5)
            .into()
    }

    fn content(&self) -> Element<'_, Self::Message>;
}
