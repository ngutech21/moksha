use std::borrow::Cow;

use iced::{
    widget::{Column, Row},
    Element, Theme,
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
