use std::{str::FromStr, time::Duration};

use chrono::Local;
use egui::TextEdit;
use email_address::EmailAddress;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
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

impl NwcApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }
}

impl eframe::App for NwcApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }
    fn auto_save_interval(&self) -> Duration {
        Duration::from_millis(5)
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if !self.wallet_connected {
            egui::Window::new("NWC")
                .resizable(false)
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label("Enter Your Nostr Wallet Connect URI:");
                    ui.separator();
                    ui.add(
                        TextEdit::multiline(&mut self.uri)
                            .hint_text("nostrwalletconnect://")
                            .id(egui::Id::new("game_name_input")),
                    );

                    ui.separator();
                    if ui.small_button("Connect Wallet").clicked() && !self.uri.is_empty() {
                        self.wallet_connected = true;
                    }
                });
        } else if self.wallet_connected {
            egui::Window::new("NWC")
                .resizable(false)
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Lightning Address: ");
                        ui.add(
                            TextEdit::singleline(&mut self.ln_address)
                                .hint_text("stutxo@zbd.gg")
                                .desired_width(150.0)
                                .id(egui::Id::new("ln_address")),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.add_space(55.0);
                        ui.label("Amount: ");
                        ui.add(
                            TextEdit::singleline(&mut self.ln_amount)
                                .hint_text("1000 sats")
                                .id(egui::Id::new("ln_amount"))
                                .desired_width(150.0),
                        );

                        if ui.small_button("send").clicked()
                            && !self.ln_address.is_empty()
                            && self.ln_amount.parse::<i32>().unwrap_or(0) > 0
                        {
                            if let Ok(ln_address) = LightningAddress::new(&self.ln_address) {
                                self.ln_address = ln_address.value.to_string();
                                let time = Local::now();
                                let formatted_time = time.format("%H:%M %d-%m-%Y");
                                let sent = format!(
                                    "{} | {} sats | {}",
                                    self.ln_address, self.ln_amount, formatted_time
                                );
                                self.history.push(sent);
                                self.ln_amount = "".to_owned();
                                self.ln_address = "".to_owned();
                            }
                        }
                    });

                    for transaction in &self.history {
                        ui.separator();
                        ui.label(transaction);
                    }

                    ui.separator();
                    ui.add_space(50.0);
                    ui.horizontal(|ui| {
                        ui.add_space(300.0);
                        if ui.small_button("Log Out").clicked() && !self.uri.is_empty() {
                            self.wallet_connected = false;
                            self.ln_address = "".to_owned();
                            self.ln_amount = "".to_owned();
                            self.uri = "".to_owned();
                            self.history.clear();
                        }
                    });
                });
        }
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
