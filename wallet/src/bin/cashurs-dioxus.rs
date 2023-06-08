use cashurs_wallet::client::HttpClient;
use cashurs_wallet::gui::components::{icon_link, icon_nav, qr_code};
use cashurs_wallet::gui::toast::{toast_frame, ToastInfo};
use cashurs_wallet::localstore::SqliteLocalStore;
use cashurs_wallet::wallet::{self, Wallet};
use dioxus_desktop::WindowBuilder;
use dioxus_router::{Route, Router, RouterContext};
use dotenvy::dotenv;

use cashurs_wallet::client::Client;

use cashurs_wallet::localstore::LocalStore;

use cashurs_wallet::gui::toast;

use dioxus::prelude::*;
use fermi::{use_atom_ref, use_init_atom_root, AtomRef};
use toast::ToastManager;

fn read_env(variable: &str) -> String {
    dotenv().expect(".env file not found");
    std::env::var(variable).expect("MINT_URL not found")
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mint_url = read_env("WALLET_MINT_URL");
    let client = HttpClient::new(mint_url.clone());
    let keys = client.get_mint_keys().await?;
    let keysets = client.get_mint_keysets().await?;

    let db_path = read_env("WALLET_DB_PATH");
    let localstore = Box::new(SqliteLocalStore::with_path(db_path).await?);

    localstore.migrate().await;

    let wallet = wallet::Wallet::new(Box::new(client), keys, keysets, localstore, mint_url);
    let balance = wallet.get_balance().await?;

    // start app
    let cfg = dioxus_desktop::Config::new()
        .with_custom_index(
            r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>Dioxus app</title>
            <meta name="viewport" content="width=device-width, initial-scale=1.0" />
            <link rel="stylesheet" href="public/bootstrap.min.css">
            <link rel="stylesheet" href="public/material.css">
            <link rel="stylesheet" href="public/app.css">
            <link rel="stylesheet" href="https://fonts.googleapis.com/icon?family=Material+Icons">
        </head>
        <body>
            <div id="main"></div>
        </body>
        </html>
        "#
            .into(),
        )
        .with_window(
            WindowBuilder::default() // FIXME include font awesome in head
                .with_title("Wallet")
                .with_resizable(true)
                .with_inner_size(dioxus_desktop::LogicalSize::new(500.0, 500.0)),
        );
    let props = AppProps { wallet, balance };

    dioxus_desktop::launch_with_props(app, props, cfg);
    Ok(())
}

static TOAST_MANAGER: AtomRef<ToastManager> = |_| ToastManager::default();

trait MyTrait {}

#[derive(Props)]
struct AppProps {
    wallet: Wallet,
    balance: u64,
}

impl PartialEq for AppProps {
    fn eq(&self, other: &Self) -> bool {
        self.balance == other.balance
    }
}

fn app(cx: Scope<AppProps>) -> Element {
    use_shared_state_provider(cx, || cx.props.wallet.clone());

    use_init_atom_root(cx);

    let toast = use_atom_ref(cx, TOAST_MANAGER);

    cx.render(rsx! {
        style {
            include_str!("../../public/bootstrap.min.css"),
            include_str!("../../public/material.css"),
            include_str!("../../public/app.css")
        }

        toast_frame { manager: toast }

        Router {
            Route { to: "/", home_page {} }
            Route { to: "/send", send_page {} }
            Route { to: "/mint", mint_page {} }
            Route { to: "/pay", pay_page {} }
            Route { to: "/receive", receive_page {} }
        }
    })
}

fn pay_page(cx: Scope) -> Element {
    cx.render(rsx! {
        div { class: "slide",
            icon_nav { icon: "arrow_back", route: "/" }
            h1 { class: "text-3xl font-bold underline", "Pay a lighting invoice" }
        }
    })
}

async fn something_async() -> usize {
    1
}

fn receive_page(cx: Scope) -> Element {
    let svc = use_context::<RouterContext>(cx).unwrap();
    let toast = use_atom_ref(cx, TOAST_MANAGER);
    let _uploaded_token: &UseRef<String> = use_ref(cx, String::new);

    cx.render(rsx! {

        div{ class: "slide",

        icon_nav { icon: "arrow_back", route: "/" }
        h1 { class: "text-3xl font-bold underline", "receive sats" }

        form {
            class: "form-group",
            onsubmit: move |event| {
                if let Some(val) = event.values.clone().get("token") {
                    let raw_token = val
                        .iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<String>>()
                        .join("");
                    if raw_token.starts_with("cashuA") {
                        svc.push_route("/", Some("home".to_owned()), None);
                        toast.write().popup(ToastInfo::success("Imported 50 Sats", "Success"));
                    } else {
                        toast.write().popup(ToastInfo::error(&format!("Invalid token: {raw_token}"), "Error"));
                    }
                }
            },
            textarea { class: "form-control", name: "token", placeholder: "Paste token here", required: true, autofocus: true }
            input {
                style: "margin-top: 5px;",
                class: "btn btn-primary mb-2",
                r#type: "submit",
                value: "Import token"
            }

            // input {
            //     class: "btn btn-primary mb-2",
            //     style: "margin-top: 5px;",
            //     r#type: "file",
            //     accept: ".txt",
            //     multiple: true,
            //     value: "Select token file",

            //     // onchange: |evt| {
            //     //     to_owned![uploaded_token];
            //     //     async move {
            //     //         if let Some(file_engine) = &evt.files {
            //     //             let files = file_engine.files();
            //     //             for file_name in &files {
            //     //                 if let Some(file) = file_engine.read_file_to_string(file_name).await {
            //     //                     //uploaded_token.write().push_str(&file);
            //     //                 }
            //     //             }
            //     //             //println!("Uploaded file: {}", uploaded_token.read());
            //     //         }
            //     //     }
            //     // }
            // }
        }
    }
    })
}
fn mint_page(cx: Scope) -> Element {
    let wallet_context = use_shared_state::<Wallet>(cx).unwrap();
    let wallet = wallet_context.read();

    let invoice = use_state(cx, || "".to_string()); // FIXME use option

    cx.render(rsx! {
        div { class: "slide",
            icon_nav { icon: "arrow_back", route: "/" }

            if !invoice.get().is_empty() {
                rsx!( qr_code { value: "{invoice}" })
             }


            button { style:"margin-top: 50px;", class: "btn btn-primary mb-2", onclick: move |_|{
                to_owned![wallet, invoice];

                async move{
                    let result = wallet.get_mint_payment_request(100).await.unwrap();
                    invoice.set(result.pr);
                }
            }, "Mint 50 Sats"
            }
        }
    })
}

fn send_page(cx: Scope) -> Element {
    let count = use_state(cx, || 0);
    cx.render(rsx! {
        div { class: "slide",

            div {
                icon_nav { icon: "arrow_back", route: "/" }
                h1 { class: "text-3xl font-bold underline", "send sats {count}" }
            }

            button { class: "btn btn-primary", onclick: |_| {
                to_owned![count];
                async move {
                    let value = something_async().await;
                    count.set(value);
                    println!("Async done {}", value);
                }
            }, "Click me" }
        }
    })
}

fn home_page(cx: Scope) -> Element {
    let wallet_context = use_shared_state::<Wallet>(cx).unwrap();
    let wallet = wallet_context.read();

    let future = use_future(cx, (), move |_| {
        let w = wallet.clone();
        async move { w.get_balance().await }
    });

    cx.render(match future.value() {
        Some(Ok(balance)) => rsx! {
                    div { class: "container",
                    h1 { class: "funds font-bold underline", "{balance} sats" }

                    div { class: "action-buttons button-bar btn-group", role: "group",
                        icon_link { title: "Mint", icon: "payments", route: "/mint" }
                        icon_link { title: "Pay", icon: "bolt", route: "/pay" }
                        icon_link { title: "Receive", icon: "download", route: "/receive" }
                        icon_link { title: "Send", icon: "send", route: "/send" }
                    }
                }
        },
        _ => {
            rsx! {
                        div { class: "container",
                        h1 { class: "funds font-bold underline", "Error loading balance" }
                    }

            }
        }
    })
}
