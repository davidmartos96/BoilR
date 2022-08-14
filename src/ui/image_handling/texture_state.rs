#[derive(Clone)]
pub enum TextureState {
    Downloading,
    Downloaded,
    Loaded(egui::TextureHandle),
}