use std::str::FromStr;

use chrono::Local;
use egui::TextEdit;
use email_address::EmailAddress;
use futures::{SinkExt, StreamExt};
use gloo_net::websocket::{futures::WebSocket, Message as WsMessage, WebSocketError};
use log::info;
use nostr::{
    nips::nip47::NostrWalletConnectURI,
    prelude::{decrypt, encrypt},
    secp256k1::{KeyPair, Message, Secp256k1, SecretKey, XOnlyPublicKey},
    ClientMessage, Event, EventId, Filter, Kind, RelayMessage, SubscriptionId, Tag, Timestamp,
};
use serde::{de::Error, Deserialize, Serialize};
use serde_json::Value;
use url::Url;
use wasm_bindgen_futures::spawn_local;

use bevy::prelude::*;
use bevy_egui::{
    egui::{self, Order},
    EguiContexts, EguiPlugin,
};

#[derive(Resource)]
pub struct NwcApp {
    uri: String,
    wallet_connected: bool,
    ln_address: String,
    ln_amount: String,
    history: Vec<String>,
}

impl Default for NwcApp {
    fn default() -> Self {
        Self {
            uri: "".to_owned(),
            wallet_connected: false,
            ln_address: "".to_owned(),
            ln_amount: "".to_owned(),
            history: Vec::new(),
        }
    }
}

fn main() {
    App::new()
        .init_resource::<NwcApp>()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "NWC-FUN".to_string(),
                // fill the entire browser window
                fit_canvas_to_parent: true,
                prevent_default_event_handling: true,
                ..default()
            }),
            ..default()
        }))
        .add_plugin(EguiPlugin)
        .add_system(nwc)
        .run();
}

fn nwc(mut contexts: EguiContexts, mut nwc: ResMut<NwcApp>) {
    let ctx = contexts.ctx_mut();

    if !nwc.wallet_connected {
        egui::Window::new("NWC")
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label("Enter Your Nostr Wallet Connect URI:");
                ui.separator();
                ui.add(
                    TextEdit::multiline(&mut nwc.uri)
                        .hint_text("nostrwalletconnect://")
                        .id(egui::Id::new("game_name_input")),
                );

                ui.separator();
                if ui.small_button("Connect Wallet").clicked() && !nwc.uri.is_empty() {
                    nwc.wallet_connected = true;
                }
            });
    } else if nwc.wallet_connected {
        egui::Window::new("NWC")
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Lightning Address: ");
                    ui.add(
                        TextEdit::singleline(&mut nwc.ln_address)
                            .hint_text("stutxo@zbd.gg")
                            .desired_width(150.0)
                            .id(egui::Id::new("ln_address")),
                    );
                });
                ui.horizontal(|ui| {
                    ui.add_space(55.0);
                    ui.label("Amount: ");
                    ui.add(
                        TextEdit::singleline(&mut nwc.ln_amount)
                            .hint_text("1000 sats")
                            .id(egui::Id::new("ln_amount"))
                            .desired_width(150.0),
                    );

                    if ui.small_button("send").clicked()
                        && !nwc.ln_address.is_empty()
                        && nwc.ln_amount.parse::<i32>().unwrap_or(0) > 0
                    {
                        if let Ok(ln_address) = LightningAddress::new(&nwc.ln_address) {
                            nwc.ln_address = ln_address.value.to_string();
                            let time = Local::now();
                            let formatted_time = time.format("%H:%M:%S %d-%m-%Y");
                            let sent = format!(
                                "{} | {} sats | {}",
                                nwc.ln_address, nwc.ln_amount, formatted_time
                            );
                            nwc.history.push(sent);
                            let uri = Url::parse(&nwc.uri).expect("Failed to parse URL");

                            // let uri = Url::parse("").expect("Failed to parse URL");

                            let relay = uri
                                .query_pairs()
                                .find(|(key, _)| key == "relay")
                                .map(|(_, value)| value.into_owned())
                                .expect("Failed to get relay");

                            let secret = uri
                                .query_pairs()
                                .find(|(key, _)| key == "secret")
                                .map(|(_, value)| value.into_owned())
                                .expect("Failed to get secret");

                            let lud16 = uri
                                .query_pairs()
                                .find(|(key, _)| key == "lud16")
                                .map(|(_, value)| value.into_owned())
                                .expect("Failed to get lud16");

                            let public_key = uri.host().unwrap().to_string();

                            let nwc_service_pubkey =
                                XOnlyPublicKey::from_str(public_key.as_str()).unwrap();
                            let secret = SecretKey::from_str(&secret).unwrap();
                            let relay_url = Url::parse(&relay).unwrap();

                            let nwc_uri = NostrWalletConnectURI::new(
                                nwc_service_pubkey,
                                relay_url,
                                Some(secret),
                                Some(lud16),
                            )
                            .unwrap();

                            let secp = Secp256k1::new();

                            let ws = WebSocket::open(&relay).unwrap();
                            let (mut write, mut read) = ws.split();

                            let nwc_key_pair = KeyPair::from_secret_key(&secp, &nwc_uri.secret);
                            let nwc_pubkey = XOnlyPublicKey::from_keypair(&nwc_key_pair);

                            //sub
                            let id = uuid::Uuid::new_v4();
                            let subscribe = ClientMessage::new_req(
                                SubscriptionId::new(id.to_string()),
                                vec![Filter::new()
                                    .kind(Kind::WalletConnectResponse)
                                    .since(Timestamp::now())
                                    .pubkey(nwc_pubkey.0)],
                            );

                            //pay

                            let amount_in_millisats: i32 = match nwc.ln_amount.parse::<i32>() {
                                Ok(val) => val * 1000,
                                Err(_) => {
                                    info!("Failed to parse ln_amount to i32");
                                    0 // Or any other default value
                                }
                            };
                            spawn_local(async move {
                                write
                                    .send(WsMessage::Text(subscribe.as_json()))
                                    .await
                                    .unwrap();
                                let ln_url = ln_address.lnurlp_url();

                                let ln_address_res = reqwest::get(ln_url).await.unwrap();

                                let body = ln_address_res.text().await.unwrap();

                                let ln_response: LnService = serde_json::from_str(&body).unwrap();

                                let callback = format!(
                                    "{}?amount={}",
                                    ln_response.callback, amount_in_millisats,
                                );

                                let invoice_res = reqwest::get(callback).await.unwrap();

                                let body = invoice_res.text().await.unwrap();
                                info!("body: {}", body);
                                let invoice: Value = serde_json::from_str(&body).unwrap();
                                let invoice = invoice["pr"]
                                    .as_str()
                                    .ok_or_else(|| serde_json::Error::custom("Missing pr field"))
                                    .unwrap();

                                let request = PayInvoiceRequest::new(invoice.to_string());

                                let created_at = Timestamp::now();
                                let kind = Kind::WalletConnectRequest;

                                let tags = vec![Tag::PubKey(nwc_service_pubkey, None)];

                                let request_bytes = serde_json::to_vec(&request).unwrap();
                                let content =
                                    encrypt(&nwc_uri.secret, &nwc_service_pubkey, &request_bytes)
                                        .unwrap();

                                let id =
                                    EventId::new(&nwc_pubkey.0, created_at, &kind, &tags, &content);

                                let id_bytes = id.as_bytes();
                                let sig = Message::from_slice(id_bytes).unwrap();

                                let pay_event = Event {
                                    id,
                                    kind,
                                    content,
                                    pubkey: nwc_pubkey.0,
                                    created_at,
                                    tags,
                                    sig: nwc_key_pair.sign_schnorr(sig),
                                };

                                let nwc_pay = ClientMessage::new_event(pay_event);
                                write
                                    .send(WsMessage::Text(nwc_pay.as_json()))
                                    .await
                                    .unwrap();

                                while let Some(web_msg) = read.next().await {
                                    match web_msg {
                                        Ok(WsMessage::Text(msg)) => {
                                            if let Ok(handled_message) =
                                                RelayMessage::from_json(msg)
                                            {
                                                match handled_message {
                                                    RelayMessage::Empty => {
                                                        info!("Empty message")
                                                    }
                                                    RelayMessage::Notice { message } => {
                                                        info!("Got a notice: {}", message);
                                                    }
                                                    RelayMessage::EndOfStoredEvents(
                                                        _subscription_id,
                                                    ) => {
                                                        info!(
                                                            "Relay signalled End of Stored Events"
                                                        );
                                                    }
                                                    RelayMessage::Ok {
                                                        event_id,
                                                        status,
                                                        message,
                                                    } => {
                                                        info!(
                                                            "Got OK message: {} - {} - {}",
                                                            event_id, status, message
                                                        );
                                                    }
                                                    RelayMessage::Event {
                                                        event,
                                                        subscription_id: _,
                                                    } => {
                                                        let event = decrypt(
                                                            &nwc_key_pair.secret_key(),
                                                            &event.pubkey,
                                                            &event.content,
                                                        )
                                                        .unwrap();

                                                        //add a match here to handle the different types of events
                                                        info!("{:#?}", event);
                                                        // return Ok(());
                                                    }
                                                    _ => (),
                                                }
                                            } else {
                                                info!("Received unexpected message");
                                            }
                                        }
                                        Ok(WsMessage::Bytes(_)) => {}

                                        Err(e) => match e {
                                            WebSocketError::ConnectionError => {}
                                            WebSocketError::ConnectionClose(_) => {}
                                            WebSocketError::MessageSendError(_) => {}
                                            _ => {}
                                        },
                                    }
                                }
                            });
                        }
                    }
                });

                let area = egui::containers::Area::new("transaction_history")
                    .order(Order::Foreground)
                    .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::new(0.0, 15.0));

                area.show(ui.ctx(), |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(50.0)
                        .max_width(300.0)
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            for transaction in &nwc.history {
                                ui.separator();
                                ui.label(transaction);
                            }
                        });
                });

                ui.separator();
                ui.add_space(100.0);
                ui.horizontal(|ui| {
                    ui.add_space(300.0);
                    if ui.small_button("Log Out").clicked() && !nwc.uri.is_empty() {
                        nwc.wallet_connected = false;
                        nwc.ln_address = "".to_owned();
                        nwc.ln_amount = "".to_owned();
                        nwc.uri = "".to_owned();
                        nwc.history.clear();
                    }
                });
            });
    }
}

#[derive(Debug, PartialEq, Clone, Eq, Hash)]
pub struct LightningAddress {
    value: EmailAddress,
}

impl LightningAddress {
    pub fn new(value: &str) -> Result<Self, String> {
        EmailAddress::from_str(value)
            .map(|value| LightningAddress { value })
            .map_err(|_| "Invalid email address".into())
    }

    #[inline]
    pub fn lnurlp_url(&self) -> String {
        format!(
            "https://{}/.well-known/lnurlp/{}",
            self.value.domain(),
            self.value.local_part()
        )
    }
}

#[derive(Deserialize, Debug)]
pub struct PayerData {
    pub name: Option<Mandatory>,
    pub identifier: Option<Mandatory>,
}

#[derive(Deserialize, Debug)]
pub struct Mandatory {
    pub mandatory: bool,
}

#[derive(Deserialize, Debug)]
pub struct LnService {
    #[serde(rename = "minSendable")]
    pub min_sendable: u64,
    #[serde(rename = "maxSendable")]
    pub max_sendable: u64,
    #[serde(rename = "commentAllowed")]
    pub comment_allowed: Option<u64>,
    pub tag: String,
    pub metadata: String,
    pub callback: String,
    #[serde(rename = "payerData")]
    pub payer_data: Option<PayerData>,
    pub disposable: Option<bool>,
    #[serde(rename = "allowsNostr")]
    pub allows_nostr: Option<bool>,
    #[serde(rename = "nostrPubkey")]
    pub nostr_pubkey: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Params {
    pub invoice: String,
}

#[derive(Serialize, Deserialize)]
pub struct PayInvoiceRequest {
    pub method: String,
    pub params: Params,
}

impl PayInvoiceRequest {
    pub fn new(invoice: String) -> Self {
        PayInvoiceRequest {
            method: "pay_invoice".to_string(),
            params: Params { invoice },
        }
    }
}
