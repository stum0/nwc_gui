#![warn(clippy::all, rust_2018_idioms)]
use log::Level;

// when compiling to web using trunk.
#[cfg(target_arch = "wasm32")]
fn main() {
    // Make sure panics are logged using `console.error`.
    console_error_panic_hook::set_once();

    // Redirect tracing to console.log and friends:
    tracing_wasm::set_as_global_default();
    console_log::init_with_level(Level::Info).expect("error initializing log");

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        eframe::start_web(
            "NWC", // hardcode it
            web_options,
            Box::new(|cc| Box::new(nwc_gui::NwcApp::new(cc))),
        )
        .await
        .expect("failed to start eframe");
    });
}
