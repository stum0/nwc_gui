use egui::TextEdit;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct TemplateApp {
    uri: String,
    wallet_connected: bool,
    ln_address: String,
    ln_amount: String,
    sent: bool,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            uri: "".to_owned(),
            wallet_connected: false,
            ln_address: "".to_owned(),
            ln_amount: "".to_owned(),
            sent: false,
        }
    }
}

impl TemplateApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }
}

impl eframe::App for TemplateApp {
    // fn save(&mut self, storage: &mut dyn eframe::Storage) {
    //     eframe::set_value(storage, eframe::APP_KEY, self);
    // }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.wallet_connected && !self.sent {
            egui::Window::new("NWC")
                .resizable(false)
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.separator();
                    ui.label("Enter Your Nostr Wallet Connect URI:");
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
        } else if self.wallet_connected && !self.sent {
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
                            self.sent = true;
                        }
                    });

                    ui.separator();
                    ui.add_space(50.0);

                    if ui.small_button("Log Out").clicked() && !self.uri.is_empty() {
                        self.wallet_connected = false;
                    }
                });
        } else if self.wallet_connected && self.sent {
            egui::Window::new("Payment Details")
                .resizable(false)
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    let payment_sent =
                        format!("{} sats sent to {}", self.ln_amount, self.ln_address);
                    ui.label(payment_sent);
                    ui.separator();
                    let preimage = "preimage";
                    ui.label(preimage);
                    ui.separator();
                    if ui.small_button("back").clicked() && !self.uri.is_empty() {
                        self.sent = false;
                    }
                });
        }
    }
}
