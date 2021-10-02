// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let app = core_of_color::App::new();
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);
}
