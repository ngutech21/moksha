use std::borrow::Cow;

use iced::{
    widget::{button, container, row, Button, Column, Container, Row, Text},
    Alignment, Element, Theme,
};

use iced::{alignment, Font, Length};

const ICON_FONT: Font = iced::Font::External {
    name: "Icons",
    bytes: include_bytes!("../../fonts/boxicons.ttf"),
};

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

pub fn btn<'a, T: 'a>(icon: Option<Text<'a>>, t: &'static str) -> Button<'a, T> {
    button::Button::new(content(icon, t))
}

fn content<'a, T: 'a>(icon: Option<Text<'a>>, t: &'static str) -> Container<'a, T> {
    match icon {
        None => container(text(t)).width(Length::Fill).center_x().padding(5),
        Some(i) => container(
            row![i, text(t)]
                .spacing(10)
                .width(iced::Length::Fill)
                .align_items(Alignment::Center),
        )
        .width(iced::Length::Fill)
        .center_x()
        .padding(5),
    }
}

fn box_icon(unicode: char) -> Text<'static> {
    Text::new(unicode.to_string())
        .font(ICON_FONT)
        //.width(Length::Units(20)) // FIXME
        .horizontal_alignment(alignment::Horizontal::Center)
        .size(20)
}

pub fn import_icon() -> Text<'static> {
    box_icon('\u{EAB9}')
}

pub fn mint_icon() -> Text<'static> {
    box_icon('\u{E9D7}')
}

pub fn receive_icon() -> Text<'static> {
    box_icon('\u{EAB9}')
}

pub fn pay_icon() -> Text<'static> {
    box_icon('\u{E9DD}')
}

// copy
// mint
// receive
// send / export token
// wallet
// settings
// mint folder EA70

// bitcoin E9D7
// bolt in circle E9DD
// bolt in cloud EA63
// bolt fat ECD1
// import cloud EA63
// export colud EA67
// settings EA6E
// dollar EAAC
// import EB1F
// export EB3E EC5F
// send telegram EB9D
// copy EBA5
// plus EBC0
// wallet EC78
