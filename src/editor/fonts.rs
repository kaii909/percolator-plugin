use egui::{self, FontData, FontDefinitions, FontFamily};
use std::sync::Arc;

pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    fonts.font_data.insert(
        "coopermed".to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../../assets/fonts/Cooper Md BT Medium.ttf"
        ))),
    );

    // cooper first, default fonts stay as fallback for missing glyphs
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "coopermed".to_owned());

    ctx.set_fonts(fonts);
}
