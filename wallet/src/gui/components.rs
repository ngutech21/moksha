#![allow(dead_code)]
use base64::{engine::general_purpose, Engine as _};
use dioxus::prelude::*;
use dioxus_router::Link;
use dioxus_std::library::clipboard::Clipboard;
use qrcode::{render::svg, QrCode};

pub fn btc_logo(cx: Scope) -> Element {
    cx.render(rsx! {
        svg {
            style: "fill: rgba(0, 0, 0, 1);transform: ;msFilter:;",
            fill: "none",
            stroke: "currentColor",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            stroke_width: "1",
            width: "48",
            height: "48",
            view_box: "0 0 48 48",
            path { d: "m11.953 8.819-.547 2.19c.619.154 2.529.784 2.838-.456.322-1.291-1.673-1.579-2.291-1.734zm-.822 3.296-.603 2.415c.743.185 3.037.921 3.376-.441.355-1.422-2.029-1.789-2.773-1.974z" }
            path { d: "M14.421 2.299C9.064.964 3.641 4.224 2.306 9.581.97 14.936 4.23 20.361 9.583 21.697c5.357 1.335 10.783-1.924 12.117-7.281 1.336-5.356-1.924-10.781-7.279-12.117zm1.991 8.275c-.145.974-.686 1.445-1.402 1.611.985.512 1.485 1.298 1.009 2.661-.592 1.691-1.998 1.834-3.87 1.48l-.454 1.82-1.096-.273.447-1.794a44.624 44.624 0 0 1-.875-.228l-.449 1.804-1.095-.275.454-1.823c-.257-.066-.517-.136-.782-.202L6.87 15l.546-1.256s.808.215.797.199c.311.077.448-.125.502-.261l.719-2.875.115.029a.864.864 0 0 0-.114-.037l.512-2.053c.013-.234-.066-.528-.511-.639.018-.011-.797-.198-.797-.198l.291-1.172 1.514.378-.001.005c.227.057.461.111.7.165l.449-1.802 1.097.273-.44 1.766c.294.067.591.135.879.207l.438-1.755 1.097.273-.449 1.802c1.384.479 2.396 1.195 2.198 2.525z" }
        }
    })
}

#[derive(Props)]
pub struct IconButtonProps<'a> {
    title: &'static str,
    icon: &'static str,
    on_click: EventHandler<'a, MouseEvent>,
}

pub fn icon_button<'a>(cx: Scope<'a, IconButtonProps<'a>>) -> Element<'a> {
    cx.render(rsx! {
        div {
            style: "display:flex;",
            class: "btn btn-primary",
            onclick: move |evt| cx.props.on_click.call(evt),
            i { class: "material-icons icon-{cx.props.icon}", "{cx.props.icon}" }
            div { "{cx.props.title}" }
        }
    })
}

#[derive(PartialEq, Props)]
pub struct IconNavProps {
    icon: &'static str,
    route: &'static str,
}

pub fn icon_nav(cx: Scope<IconNavProps>) -> Element {
    cx.render(rsx! {
        Link {
            to: cx.props.route,
            class: "nav-button material-icons icon-{cx.props.icon}",
            "{cx.props.icon}"
        }
    })
}

#[derive(PartialEq, Props)]
pub struct IconLinkProps {
    title: &'static str,
    icon: &'static str,
    route: &'static str,
}

pub fn icon_link(cx: Scope<IconLinkProps>) -> Element {
    cx.render(rsx! {
        Link { to: cx.props.route,
            div { style: "display:flex; padding 5px; margin: 5px;", class: "btn btn-primary",
                i {
                    style: "margin-right: 5px;",
                    class: "material-icons icon-{cx.props.icon}",
                    "{cx.props.icon}"
                }
                div { "{cx.props.title}" }
            }
        }
    })
}

#[derive(PartialEq, Props)]
pub struct QrCodeProps<'a> {
    value: &'a str,
}

//pub fn icon_button<'a>(cx: Scope<'a, IconButtonProps<'a>>) -> Element<'a> {

pub fn qr_code<'a>(cx: Scope<'a, QrCodeProps<'a>>) -> Element<'a> {
    let raw_value = cx.props.value.to_string();
    let qrdata = generate_qrcode(raw_value.clone());
    let mut clp = Clipboard::new().unwrap(); // FIXME handle error
    let _res = clp.set_content(raw_value); // FIXME handle error

    cx.render(rsx! {
        div {
            class: "card position-absolute top-50 start-50 translate-middle",
            style: "width: 18rem;",
            img { class: "card-img-top", alt: "", src: "{qrdata}" }
            div { class: "card-body red-text", h6 { class: "card-title", "Scan this QR code to mint sats" } }
        }
    })
}

fn generate_qrcode(value: String) -> String {
    let code = QrCode::new(value.as_bytes()).unwrap(); // FIXME: handle error
    let image = code
        .render()
        .min_dimensions(150, 150)
        .dark_color(svg::Color("#000000"))
        .light_color(svg::Color("#ffffff"))
        .build();
    format!(
        "data:image/svg+xml;base64,{}",
        general_purpose::STANDARD.encode(image)
    )
}
