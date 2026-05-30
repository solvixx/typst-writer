use crate::ui::workspace::EditorWorkspace;
use gpui::*;
use gpui_component::v_flex;

pub struct AssetPanel {
    workspace: WeakEntity<EditorWorkspace>,
}

impl AssetPanel {
    pub fn new(workspace: WeakEntity<EditorWorkspace>) -> Self {
        Self { workspace }
    }
}

impl Render for AssetPanel {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let _ws = self.workspace.upgrade();

        div().size_full().bg(rgb(0x0f172a)).p_4().child(
            v_flex()
                .gap_4()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(0x94a3b8))
                        .child("PROJECT ASSETS"),
                )
                .child(
                    v_flex()
                        .gap_2()
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x64748b))
                                .child("Drop .ttf / .otf fonts here to add to project"),
                        )
                        .child(
                            div()
                                .h_20()
                                .w_full()
                                .border_1()
                                .border_color(rgb(0x1e293b))
                                .rounded_md()
                                .flex()
                                .items_center()
                                .justify_center()
                                .bg(rgb(0x020617))
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(0x475569))
                                        .child("Dropzone placeholder"),
                                ),
                        ),
                ),
        )
    }
}

impl Focusable for AssetPanel {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        cx.focus_handle()
    }
}

impl gpui_component::dock::Panel for AssetPanel {
    fn panel_name(&self) -> &'static str {
        "AssetPanel"
    }

    fn title(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        "Assets"
    }
}

impl EventEmitter<gpui_component::dock::PanelEvent> for AssetPanel {}
