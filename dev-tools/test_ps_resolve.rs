use gpui::*;

fn main() {
    Application::new().run(|cx| {
        let mut fonts = Vec::new();
        for font_bytes in typst_assets::fonts() {
            fonts.push(std::borrow::Cow::Borrowed(font_bytes));
        }
        let _ = cx.text_system().add_fonts(fonts);

        let ps_names = [
            "LibertinusSerif-Regular",
            "LibertinusSerif-Bold",
            "LibertinusSerif-Semibold",
            "LibertinusSerif-SemiboldItalic",
            "NewCMMath-Book",
            "NewCMMath-Regular",
        ];

        for ps in ps_names {
            let id = cx.text_system().resolve_font(&Font {
                family: ps.into(),
                weight: FontWeight::NORMAL,
                style: FontStyle::Normal,
                features: Default::default(),
                fallbacks: None,
            });
            println!("Resolved PS '{}' -> {:?}", ps, id);
        }

        cx.quit();
    });
}
