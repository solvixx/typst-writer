use gpui::*;
use typst_writer::ui::workspace::EditorWorkspace;
use typst_writer::ui::load_ui_fonts;
use std::borrow::Cow;

struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        match path {
            // window
            "icons/window/window-close.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/window/window-close.svg")))),
            "icons/window/window-maximize.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/window/window-maximize.svg")))),
            "icons/window/window-minimize.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/window/window-minimize.svg")))),
            "icons/window/window-restore.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/window/window-restore.svg")))),
            // editor
            "icons/editor/minus.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/editor/minus.svg")))),
            "icons/editor/plus.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/editor/plus.svg")))),
            "icons/editor/undo.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/editor/undo.svg")))),
            "icons/editor/bold.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/editor/bold.svg")))),
            "icons/editor/italic.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/editor/italic.svg")))),
            "icons/editor/heading.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/editor/heading.svg")))),
            "icons/editor/list.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/editor/list.svg")))),
            "icons/editor/code.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/editor/code.svg")))),
            // math
            "icons/math/pi.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/math/pi.svg")))),
            "icons/math/fraction.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/math/fraction.svg")))),
            "icons/math/sqrt.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/math/sqrt.svg")))),
            "icons/math/sigma.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/math/sigma.svg")))),
            "icons/math/integral.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/math/integral.svg")))),
            "icons/math/variable.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/math/variable.svg")))),
            "icons/math/matrix.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/math/matrix.svg")))),
            "icons/math/superscript.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/math/superscript.svg")))),
            "icons/math/subscript.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/math/subscript.svg")))),
            "icons/math/alpha.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/math/alpha.svg")))),
            "icons/math/alpha-upper.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/math/alpha-upper.svg")))),
            "icons/math/beta.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/math/beta.svg")))),
            "icons/math/beta-upper.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/math/beta-upper.svg")))),
            "icons/math/gamma.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/math/gamma.svg")))),
            "icons/math/gamma-upper.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/math/gamma-upper.svg")))),
            "icons/math/theta.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/math/theta.svg")))),
            "icons/math/theta-upper.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/math/theta-upper.svg")))),
            "icons/math/omega.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/math/omega.svg")))),
            "icons/math/omega-upper.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/math/omega-upper.svg")))),
            "icons/math/pi-upper.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/math/pi-upper.svg")))),
            // ui
            "icons/ui/settings-2.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/ui/settings-2.svg")))),
            "icons/ui/eye.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/ui/eye.svg")))),
            "icons/ui/home.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/ui/home.svg")))),
            "icons/ui/plus-square.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/ui/plus-square.svg")))),
            "icons/ui/function.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/ui/function.svg")))),
            "icons/ui/download.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/ui/download.svg")))),
            "icons/ui/bug.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/ui/bug.svg")))),
            // additional icons
            "icons/folder-open.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/folder-open.svg")))),
            "icons/layout-dashboard.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/layout-dashboard.svg")))),
            "icons/minimize.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/minimize.svg")))),
            "icons/maximize.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/maximize.svg")))),
            "icons/ellipsis.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/ellipsis.svg")))),
            "icons/panel-left.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/panel-left.svg")))),
            "icons/panel-left-open.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/panel-left-open.svg")))),
            "icons/panel-right.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/panel-right.svg")))),
            "icons/panel-right-open.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/panel-right-open.svg")))),
            "icons/panel-bottom.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/panel-bottom.svg")))),
            "icons/panel-bottom-open.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/panel-bottom-open.svg")))),
            "icons/menu.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/menu.svg")))),
            "icons/file.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/file.svg")))),
            "icons/folder.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/folder.svg")))),
            "icons/folder-closed.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/folder-closed.svg")))),
            "icons/save.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/save.svg")))),
            "icons/file-plus.svg" => Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/file-plus.svg")))),
            _ => Ok(None),
        }
    }

    fn list(&self, _path: &str) -> Result<Vec<SharedString>> {
        Ok(vec![])
    }
}

fn main() {
    Application::new().with_assets(Assets).run(|cx: &mut App| {
        gpui_component::init(cx);
        
        load_ui_fonts(cx);

        let bounds = Bounds::centered(None, size(px(1200.0), px(800.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                #[cfg(target_os = "linux")]
                window_background: gpui::WindowBackgroundAppearance::Transparent,
                window_decorations: Some(gpui::WindowDecorations::Client),
                ..Default::default()
            },
            |window, cx| {
                let sample_doc = "= Page 1: Native WYSIWYG Layout\n\nThis is a true hardware-accelerated editor running on GPUI.\n\nTry clicking and dragging text here to see dynamic translucent highlights!\n\n#pagebreak()\n= Page 2: Mathematical Expressions\n\nInline equations like $a^2 + b^2 = c^2$ compile on every keystroke. Below is a block matrix:\n\n$ d / (d x) integral_a^x f(t) d t = f(x) $\n\nTry clicking math elements or writing code blocks to see live adaptive ribbon changes!\n$ sqrt(2 x + 1) $".to_string();
                let view = cx.new(|cx| EditorWorkspace::new(window, cx, &sample_doc));
                let window_handle = window.window_handle();
                view.update(cx, |this, _cx| {
                    this.window_handle = window_handle;
                    window.focus(&this.focus_handle);
                });
                cx.new(|cx| gpui_component::Root::new(view, window, cx))
            },
        )
        .unwrap();

        cx.on_window_closed(|cx| {
            cx.quit();
        })
        .detach();
        cx.activate(true);
    });
}
