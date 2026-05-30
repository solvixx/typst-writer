use gpui::*;
use gpui_component::h_flex;
use gpui_component::TitleBar as GpuiTitleBar;
use gpui_component::ActiveTheme;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::IconName;
use crate::ui::workspace::EditorWorkspace;

pub struct TitleBar {
    _workspace: WeakEntity<EditorWorkspace>,
}

impl TitleBar {
    pub fn new(workspace: WeakEntity<EditorWorkspace>, _cx: &mut Context<Self>) -> Self {
        Self { _workspace: workspace }
    }
}

impl Render for TitleBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let workspace = self._workspace.clone();
        
        GpuiTitleBar::new()
            .child(
                h_flex()
                    .items_center()
                    .px_2()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::BOLD)
                            .child("Typst Writer")
                    )
                    .child(
                        div()
                            .ml_4()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("v0.1.0")
                    )
                    .child(
                        div()
                            .ml_4()
                            .child(
                                Button::new("new-document")
                                    .icon(IconName::FilePlus)
                                    .ghost()
                                    .compact()
                                    .tooltip("New Document")
                                    .on_click({
                                        let workspace = workspace.clone();
                                        move |_, window, cx| {
                                            if let Some(ws) = workspace.upgrade() {
                                                ws.update(cx, |this, cx| this.new_document(window, cx));
                                            }
                                        }
                                    })
                            )
                    )
                    .child(
                        div()
                            .ml_2()
                            .child(
                                Button::new("open-folder")
                                    .icon(IconName::FolderOpen)
                                    .ghost()
                                    .compact()
                                    .tooltip("Open Folder")
                                    .on_click({
                                        let workspace = workspace.clone();
                                        move |_, window, cx| {
                                            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                                if let Some(ws) = workspace.upgrade() {
                                                    window.defer(cx, move |window, cx| {
                                                        ws.update(cx, |this, cx| this.change_working_directory(path.clone(), window, cx));
                                                    });
                                                }
                                            }
                                        }
                                    })
                            )
                    )
                    .child(
                        div()
                            .ml_2()
                            .child(
                                Button::new("open-preferences")
                                    .icon(IconName::Settings)
                                    .ghost()
                                    .compact()
                                    .tooltip("Preferences")
                                    .on_click({
                                        let workspace = workspace.clone();
                                        move |_, window, cx| {
                                            if let Some(ws) = workspace.upgrade() {
                                                ws.update(cx, |this, cx| this.open_preferences(window, cx));
                                            }
                                        }
                                    })
                            )
                    )
            )
    }
}
