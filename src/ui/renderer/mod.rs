pub mod geometry;
pub mod painter;

use gpui::*;
use typst::layout::PagedDocument;
use crate::ui::workspace::EditorWorkspace;
use gpui_component::{RopeExt, v_flex, h_flex, IconName, Sizable, ActiveTheme};
use gpui_component::button::{Button, ButtonVariants};
use crate::ui::renderer::geometry::PT_TO_PX;
use gpui_component::menu::{ContextMenuExt, PopupMenuItem};

/// The RendererView component provides a WYSIWYG (What You See Is What You Get) preview
/// of the Typst document. It handles rendering, spatial navigation, and text selection
/// directly on the rendered pages.
pub struct RendererView {
    /// The parent workspace handle.
    workspace: WeakEntity<EditorWorkspace>,
    /// Focus handle for keyboard input.
    focus_handle: FocusHandle,
    /// Vertical offsets of each page in the layout.
    page_tops: Vec<Pixels>,
    /// Maximum width of the document in pixels.
    max_doc_width: Pixels,
    /// Handle for managing scroll state.
    scroll_handle: ScrollHandle,
    /// Zoom level (1.0 = 100%).
    zoom: f32,
    /// State for the zoom slider component.
    zoom_slider_state: Entity<gpui_component::slider::SliderState>,
    
    /// SNAPSHOT FIELDS for 60fps rendering without entity locking
    /// The currently compiled document.
    compiled_document: Option<std::sync::Arc<PagedDocument>>,
    /// Error message from the last compilation, if any.
    error_message: Option<String>,
    /// Version of the last successfully rendered document.
    last_document_version: Option<usize>,
    #[cfg(debug_assertions)]
    /// Whether to show glyph bounding boxes (debug only).
    show_glyph_boxes: bool,
    
    /// Animation state: timestamp of the last keystroke, used for caret blinking.
    last_keystroke_at: std::time::Instant,
    
    /// IME Composition state: start offset of the current composition.
    composition_offset: Option<usize>,
    /// IME Composition state: length of the current composition.
    composition_length: usize,
    
    /// Persistent font cache for performance.
    font_cache: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<u64, gpui::FontId>>>,

    /// Window-space origin of each page's canvas element, captured during paint.
    /// Used to convert window-space mouse events into page-local glyph-box coordinates.
    page_canvas_origins: std::sync::Arc<std::sync::Mutex<Vec<Point<Pixels>>>>,
}

impl gpui::EntityInputHandler for RendererView {
    fn text_for_range(
        &mut self,
        range: std::ops::Range<usize>,
        _adjusted_range: &mut Option<std::ops::Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<String> {
        if let Some(ws_handle) = self.workspace.upgrade() {
            let ws = ws_handle.read(cx);
            let text = ws.world.source_ref().text();
            let end = range.end.min(text.len());
            let start = range.start.min(end);
            Some(text[start..end].to_string())
        } else {
            None
        }
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<gpui::UTF16Selection> {
        if let Some(ws_handle) = self.workspace.upgrade() {
            let ws = ws_handle.read(cx);
            Some(gpui::UTF16Selection {
                range: ws.cursor_offset..ws.cursor_offset,
                reversed: false,
            })
        } else {
            None
        }
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<std::ops::Range<usize>> {
        self.composition_offset.map(|offset| offset..offset + self.composition_length)
    }

    fn unmark_text(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(ws_handle) = self.composition_offset.filter(|_| self.composition_length > 0).and_then(|_| self.workspace.upgrade()) {
            let offset = self.composition_offset.unwrap();
            ws_handle.update(cx, |ws, cx| {
                ws.apply_virtual_editor_action(crate::core::editor::EditorAction::Edit {
                    range: offset..offset + self.composition_length,
                    replacement: String::new(),
                    new_cursor: offset,
                    new_selection: None,
                }, window, cx);
            });
        }
        self.composition_offset = None;
        self.composition_length = 0;
        cx.notify();
    }

    fn replace_text_in_range(
        &mut self,
        _range: Option<std::ops::Range<usize>>,
        text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(ws_handle) = self.workspace.upgrade() {
            ws_handle.update(cx, |ws, cx| {
                if let Some(offset) = self.composition_offset {
                    ws.apply_editor_action(crate::core::editor::EditorAction::Edit {
                        range: offset..offset + self.composition_length,
                        replacement: text.to_string(),
                        new_cursor: offset + text.len(),
                        new_selection: None,
                    }, _window, cx);
                } else {
                    if text.chars().count() == 1 {
                        let source = ws.world.source_ref();
                        let state = crate::core::editor::EditorState {
                            text: source.text(),
                            cursor: ws.cursor_offset,
                            selection: ws.selection.map(|s| s.start..s.end),
                            context: ws.editor_context,
                        };
                        let action = crate::core::renderer::keys::wysiwyg_key_event(
                            source,
                            &state,
                            text,
                            false,
                        );
                        if action != crate::core::editor::EditorAction::None {
                            ws.apply_editor_action(action, _window, cx);
                        }
                    } else {
                        let offset = ws.cursor_offset;
                        let range = if let Some(sel) = ws.selection {
                            let start = sel.start.min(sel.end);
                            let end = sel.start.max(sel.end);
                            start..end
                        } else {
                            offset..offset
                        };
                        ws.apply_editor_action(crate::core::editor::EditorAction::Edit {
                            range: range.clone(),
                            replacement: text.to_string(),
                            new_cursor: range.start + text.len(),
                            new_selection: None,
                        }, _window, cx);
                    }
                }
            });
        }
        self.composition_offset = None;
        self.composition_length = 0;
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        _range: Option<std::ops::Range<usize>>,
        new_text: &str,
        _new_selected_range: Option<std::ops::Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(ws_handle) = self.workspace.upgrade() {
            ws_handle.update(cx, |ws, cx| {
                let offset = self.composition_offset.unwrap_or(ws.cursor_offset);
                self.composition_offset = Some(offset);
                
                ws.apply_virtual_editor_action(crate::core::editor::EditorAction::Edit {
                    range: offset..offset + self.composition_length,
                    replacement: new_text.to_string(),
                    new_cursor: offset + new_text.len(),
                    new_selection: None,
                }, _window, cx);
                
                self.composition_length = new_text.len();
            });
        }
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        _range_utf16: std::ops::Range<usize>,
        _element_bounds: Bounds<Pixels>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        if let Some(ws_handle) = self.workspace.upgrade() {
            let ws = ws_handle.read(cx);
            if let Some(((caret_idx, rel_cursor, cursor_h_px), doc)) = ws.cursor_relative_pos.zip(self.compiled_document.as_ref()) {
                let origin = self.calculate_page_origin(caret_idx, doc);
                let zoom = self.zoom;
                let cursor_h = px(cursor_h_px * zoom * 0.9);
                let caret_top = rel_cursor.y * zoom - cursor_h * 0.82;
                
                return Some(Bounds {
                    origin: point(origin.x + rel_cursor.x * zoom, origin.y + caret_top),
                    size: size(px(0.0), cursor_h),
                });
            }
        }
        None
    }

    fn character_index_for_point(
        &mut self,
        _point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        None
    }
}

impl RendererView {
    pub fn new(workspace: WeakEntity<EditorWorkspace>, cx: &mut Context<Self>) -> Self {
        // Repaint when the workspace notifies changes
        if let Some(ws_handle) = workspace.upgrade() {
            cx.observe(&ws_handle, |this, ws_handle, cx| {
                this.last_keystroke_at = std::time::Instant::now();
                
                // Read snapshots to release borrow immediately
                let (doc_version, compiled_doc, _world_source, error_msg, _show_boxes) = {
                    let ws = ws_handle.read(cx);
                    #[cfg(debug_assertions)]
                    let show_boxes = ws.show_glyph_boxes;
                    #[cfg(not(debug_assertions))]
                    let show_boxes = false;
                    (
                        ws.document_version,
                        ws.compiled_document.clone(),
                        ws.world.source_ref().clone(),
                        ws.error_message.clone(),
                        show_boxes,
                    )
                };
                
                // Update basic snapshots
                if this.error_message != error_msg {
                    this.error_message = error_msg.clone();
                }

                #[cfg(debug_assertions)]
                if this.show_glyph_boxes != _show_boxes {
                    this.show_glyph_boxes = _show_boxes;
                }

                // PERFORMANCE: Update visual data immediately, offload hit-testing metrics to background
                if Some(doc_version) != this.last_document_version {
                    if let Some(doc) = compiled_doc.clone() {
                        let version = doc_version;
                        let _this_weak = cx.entity().downgrade();
                        
                        this.compiled_document = Some(doc);
                        this.recalculate_layout();
                        this.last_document_version = Some(version);
                    } else if this.compiled_document.is_some() {
                        this.compiled_document = None;
                        
                        let ws_handle = this.workspace.upgrade().expect("Workspace dropped");
                        ws_handle.update(cx, |ws, _cx| {
                            ws.refresh_caret_location(_cx, false);
                        });

                        this.max_doc_width = px(0.0);
                        this.last_document_version = Some(doc_version);
                    }
                }

                cx.notify();
            }).detach();
        }

        let focus_handle = cx.focus_handle();

        cx.bind_keys([
            #[cfg(target_os = "macos")]
            KeyBinding::new("cmd-c", gpui_component::input::Copy, Some("Renderer")),
            #[cfg(not(target_os = "macos"))]
            KeyBinding::new("ctrl-c", gpui_component::input::Copy, Some("Renderer")),
            #[cfg(target_os = "macos")]
            KeyBinding::new("cmd-x", gpui_component::input::Cut, Some("Renderer")),
            #[cfg(not(target_os = "macos"))]
            KeyBinding::new("ctrl-x", gpui_component::input::Cut, Some("Renderer")),
            #[cfg(target_os = "macos")]
            KeyBinding::new("cmd-v", gpui_component::input::Paste, Some("Renderer")),
            #[cfg(not(target_os = "macos"))]
            KeyBinding::new("ctrl-v", gpui_component::input::Paste, Some("Renderer")),
            #[cfg(target_os = "macos")]
            KeyBinding::new("cmd-z", gpui_component::input::Undo, Some("Renderer")),
            #[cfg(target_os = "macos")]
            KeyBinding::new("cmd-shift-z", gpui_component::input::Redo, Some("Renderer")),
            #[cfg(not(target_os = "macos"))]
            KeyBinding::new("ctrl-z", gpui_component::input::Undo, Some("Renderer")),
            #[cfg(not(target_os = "macos"))]
            KeyBinding::new("ctrl-y", gpui_component::input::Redo, Some("Renderer")),
            KeyBinding::new("home", gpui_component::input::MoveHome, Some("Renderer")),
            KeyBinding::new("end", gpui_component::input::MoveEnd, Some("Renderer")),
            KeyBinding::new("pageup", gpui_component::input::MovePageUp, Some("Renderer")),
            KeyBinding::new("pagedown", gpui_component::input::MovePageDown, Some("Renderer")),
        ]);

        let this = Self { 
            workspace, 
            focus_handle, 
            page_tops: Vec::new(),
            max_doc_width: px(0.0),
            scroll_handle: ScrollHandle::new(),
            zoom: 1.0,
            compiled_document: None,
            error_message: None,
            last_document_version: None,
            #[cfg(debug_assertions)]
            show_glyph_boxes: false,
            last_keystroke_at: std::time::Instant::now(),
            composition_offset: None,
            composition_length: 0,
            font_cache: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            page_canvas_origins: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            zoom_slider_state: cx.new(|_cx| {
                gpui_component::slider::SliderState::new()
                    .min(0.2)
                    .max(5.0)
                    .default_value(1.0)
                    .step(0.01)
            }),
        };

        cx.subscribe(&this.zoom_slider_state, |this, _state, event: &gpui_component::slider::SliderEvent, cx| {
            match event {
                gpui_component::slider::SliderEvent::Change(val) => {
                    this.zoom = val.end();
                    this.recalculate_layout();
                    cx.notify();
                }
            }
        }).detach();

        this
    }

    fn handle_mouse_down(&mut self, ev: &gpui::MouseDownEvent, window: &mut Window, cx: &mut Context<Self>, page_idx: usize) {
        self.last_keystroke_at = std::time::Instant::now();
        self.focus_handle.focus(window);
        let ws_handle = self.workspace.upgrade().expect("Workspace dropped");
        // ev.position is in window space. Subtract the canvas origin (also window space)
        // to get page-local coordinates that match the zoomed glyph-box space.
        let page_origin = self.page_canvas_origins
            .lock().unwrap()
            .get(page_idx).cloned().unwrap_or_default();
        let rel_click = point(ev.position.x - page_origin.x, ev.position.y - page_origin.y);
        let zoom = self.zoom;
        ws_handle.update(cx, |this, cx| {
            if let Some(doc) = &this.compiled_document {
                if let Some(page) = doc.pages.get(page_idx) {
                    let mut boxes = Vec::new();
                    geometry::collect_glyph_boxes_with_source(&page.frame, Point::default(), this.world.source_ref(), &mut boxes, zoom);
                    if let Some((raw_offset, _, _)) = geometry::find_closest_offset(&boxes, rel_click, this.world.source_ref()) {
                    let source_len = this.world.source_ref().text().len();
                    
                    let editor_len = if let Some(active_path) = &this.active_editor_path {
                        this.editors.get(active_path).map(|e| e.read(cx).input.read(cx).text().len()).unwrap_or(source_len)
                    } else {
                        source_len
                    };
                    
                    let offset = raw_offset.min(source_len).min(editor_len);

                    #[cfg(debug_assertions)]
                    {
                        if let Some(glyph_box) = boxes.iter().find(|b| b.offset == raw_offset) {
                            let symbol = if glyph_box.is_radical {
                                let node_text = this.world.source_ref().text().get(offset..).unwrap_or("");
                                if node_text.starts_with("sqrt") || node_text.starts_with('√') {
                                    '√'
                                } else if node_text.starts_with("root") || node_text.starts_with('∛') || node_text.starts_with('∜') {
                                    '∛'
                                } else if node_text.starts_with("integral") || node_text.starts_with("int") || node_text.starts_with('∫') || node_text.starts_with('∬') || node_text.starts_with('∭') || node_text.starts_with('∮') {
                                    '∫'
                                } else {
                                    node_text.chars().next().unwrap_or('√')
                                }
                            } else {
                                this.world.source_ref().text().get(offset..).and_then(|s| s.chars().next()).unwrap_or('?')
                            };
                            this.log("DEBUG", format!("Clicked: '{}' (offset={}), rel_click={:?}, bounds={:?}, height={}, baseline={:?}, is_text={}", 
                                symbol, glyph_box.offset, rel_click, glyph_box.bounds, glyph_box.height, glyph_box.baseline, glyph_box.is_text));
                        }
                    }
                    this.apply_editor_action(crate::core::editor::EditorAction::MoveCursor { new_cursor: offset }, window, cx);
                    this.selection = Some(gpui_component::input::Selection::new(offset, offset));
                        this.is_dragging = true;
                    }
                }
            }
            cx.notify();
        });
    }

    fn handle_mouse_move(&mut self, ev: &gpui::MouseMoveEvent, window: &mut Window, cx: &mut Context<Self>, page_idx: usize) {
        let ws_handle = self.workspace.upgrade().expect("Workspace dropped");
        // Same window-space → page-local conversion as mouse_down.
        let page_origin = self.page_canvas_origins
            .lock().unwrap()
            .get(page_idx).cloned().unwrap_or_default();
        let rel_click = point(ev.position.x - page_origin.x, ev.position.y - page_origin.y);
        let zoom = self.zoom;
        ws_handle.update(cx, |this_ws, cx| {
            if !this_ws.is_dragging { return; }
            if let Some(doc) = &this_ws.compiled_document {
                if let Some(page) = doc.pages.get(page_idx) {
                    let mut boxes = Vec::new();
                    geometry::collect_glyph_boxes_with_source(&page.frame, Point::default(), this_ws.world.source_ref(), &mut boxes, zoom);
                    if let Some((raw_offset, _, _)) = geometry::find_closest_offset(&boxes, rel_click, this_ws.world.source_ref()) {
                    let source_len = this_ws.world.source_ref().text().len();
                    
                    let editor_len = if let Some(active_path) = &this_ws.active_editor_path {
                        this_ws.editors.get(active_path).map(|e| e.read(cx).input.read(cx).text().len()).unwrap_or(source_len)
                    } else {
                        source_len
                    };
                    
                    let offset = raw_offset.min(source_len).min(editor_len);

                    // PERFORMANCE: Manual sync to SourceEditor to preserve dragging state (selection)
                    // without clearing selection_start as MoveCursor action would do.
                    this_ws.cursor_offset = offset;
                    let mut anchor = offset;
                    if let Some(ref mut sel) = this_ws.selection {
                        sel.start = sel.start.min(editor_len);
                        sel.end = sel.end.min(editor_len);
                        anchor = sel.start;
                        sel.end = offset;
                    } else {
                        this_ws.selection = Some(gpui_component::input::Selection::new(offset, offset));
                    }

                    let target_sel = if offset < anchor {
                        gpui_component::input::Selection::new(offset, anchor)
                    } else {
                        gpui_component::input::Selection::new(anchor, offset)
                    };
                    
                    // Sync to SourceEditor
                    let previous_focus = window.focused(cx);
                    let active_editor = this_ws.active_editor_path.as_ref()
                        .and_then(|path| this_ws.editors.get(path).cloned());
                    
                    if let Some(view) = active_editor {
                        let editor_focus = view.read(cx).focus_handle(cx);
                        view.update(cx, |view, cx| {
                            view.input.update(cx, |state, cx| {
                                let pos = state.text().offset_to_position(offset);
                                state.set_cursor_position(pos, window, cx);
                                state.set_selected_range(target_sel, offset < anchor, cx);
                            });
                        });
                        
                        // Restore focus if stolen
                        if let Some(prev) = previous_focus.filter(|p| *p != editor_focus && editor_focus.is_focused(window)) {
                            prev.focus(window);
                        }
                    }

                    this_ws.refresh_caret_location(cx, true);
                    }
                }
            }
            cx.notify();
        });
    }

    fn handle_mouse_up(&mut self, cx: &mut Context<Self>) {
        let ws_handle = self.workspace.upgrade().expect("Workspace dropped");
        ws_handle.update(cx, |this_ws, cx| {
            this_ws.is_dragging = false;
            cx.notify();
        });
    }

    fn handle_key_down(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.last_keystroke_at = std::time::Instant::now();
        let ws_handle = self.workspace.upgrade().expect("Workspace dropped");
        
        let key = &ev.keystroke.key;
        
        let is_nav = matches!(key.as_str(), "up" | "down" | "left" | "right" | "home" | "end" | "pageup" | "pagedown");

        #[cfg(debug_assertions)]
        if is_nav {
            ws_handle.update(cx, |this, _| {
                this.log("DEBUG", format!("Navigation key pressed: {}", key));
            });
        }
        let is_control = matches!(key.as_str(), "backspace" | "delete" | "enter" | "tab");
        
        if is_nav && let Some((page_idx, pos, _height)) = self.compiled_document.as_ref().and_then(|_| ws_handle.read(cx).cursor_relative_pos) {
            let target_page = page_idx;
            let target_pos = pos;
            let mut found = false;

            if let Some(doc) = self.compiled_document.as_ref() {
                if let Some(page) = doc.pages.get(target_page) {
                    let mut boxes = Vec::new();
                    geometry::collect_glyph_boxes_with_source(&page.frame, Point::default(), ws_handle.read(cx).world.source_ref(), &mut boxes, self.zoom);
                    
                    let zoomed_pos = point(target_pos.x * self.zoom, target_pos.y * self.zoom);
                    let current_offset = ws_handle.read(cx).cursor_offset;
                    if let Some((new_offset, _new_pos, _new_h)) = geometry::move_in_direction(&boxes, zoomed_pos, key, current_offset, ws_handle.read(cx).world.source_ref()) {
                        ws_handle.update(cx, |ws, cx| {
                            if ev.keystroke.modifiers.shift {
                                let anchor = if let Some(sel) = &ws.selection {
                                    if ws.cursor_offset == sel.start { sel.end } else { sel.start }
                                } else {
                                    ws.cursor_offset
                                };
                                let range = if new_offset < anchor { new_offset..anchor } else { anchor..new_offset };
                                ws.apply_editor_action(crate::core::editor::EditorAction::Select { 
                                    range, 
                                    reversed: new_offset < anchor 
                                }, window, cx);
                            } else {
                                ws.apply_editor_action(crate::core::editor::EditorAction::MoveCursor { new_cursor: new_offset }, window, cx);
                            }
                        });
                        found = true;
                    }
                }
            }

            // Cross-page navigation for Up/Down
            if !found && (key == "up" || key == "down") {
                let next_page = if key == "up" { target_page.checked_sub(1) } else { Some(target_page + 1) };
                if let Some(p) = next_page {
                    if let Some(doc) = self.compiled_document.as_ref() {
                        if let Some(page) = doc.pages.get(p) {
                            let mut boxes = Vec::new();
                            geometry::collect_glyph_boxes_with_source(&page.frame, Point::default(), ws_handle.read(cx).world.source_ref(), &mut boxes, self.zoom);
                            
                            // Start from the opposite edge of the next page (zoomed)
                            let start_y = if key == "up" { px(page.frame.height().to_pt() as f32 * PT_TO_PX * self.zoom) } else { px(0.0) };
                            let start_pos = point(target_pos.x * self.zoom, start_y);
                            let current_offset = ws_handle.read(cx).cursor_offset;
                            
                            if let Some((new_offset, _, _)) = geometry::move_in_direction(&boxes, start_pos, key, current_offset, ws_handle.read(cx).world.source_ref()) {
                                ws_handle.update(cx, |ws, cx| {
                                    if ev.keystroke.modifiers.shift {
                                        let anchor = if let Some(sel) = &ws.selection {
                                            if ws.cursor_offset == sel.start { sel.end } else { sel.start }
                                        } else {
                                            ws.cursor_offset
                                        };
                                        let range = if new_offset < anchor { new_offset..anchor } else { anchor..new_offset };
                                        ws.apply_editor_action(crate::core::editor::EditorAction::Select { 
                                            range, 
                                            reversed: new_offset < anchor 
                                        }, window, cx);
                                    } else {
                                        ws.apply_editor_action(crate::core::editor::EditorAction::MoveCursor { new_cursor: new_offset }, window, cx);
                                    }
                                });
                                found = true;
                            }
                        }
                    }
                }
            }

            if found { return; }
        }

        if is_nav || is_control {
            ws_handle.update(cx, |ws, cx| {
                let source = ws.world.source_ref();
                let state = crate::core::editor::EditorState {
                    text: source.text(),
                    cursor: ws.cursor_offset,
                    selection: ws.selection.map(|s| s.start..s.end),
                    context: ws.editor_context,
                };
                
                let action = crate::core::renderer::keys::wysiwyg_key_event(
                    source,
                    &state,
                    key,
                    ev.keystroke.modifiers.shift,
                );
                
                if action != crate::core::editor::EditorAction::None {
                    ws.apply_editor_action(action, window, cx);
                }
            });
        }
    }

    fn calculate_page_origin(&self, page_idx: usize, doc: &PagedDocument) -> Point<Pixels> {
        let zoom = self.zoom;
        let viewport_bounds = self.scroll_handle.bounds();
        let scroll_offset = self.scroll_handle.offset();
        
        let page_width = px(doc.pages[page_idx].frame.width().to_pt() as f32 * PT_TO_PX * zoom);
        let page_top = self.page_tops.get(page_idx).cloned().unwrap_or(px(0.0));
        
        // PERFORMANCE: content_width is the width of the v_flex container
        let content_width = self.max_doc_width.max(viewport_bounds.size.width);
        // padding p_6 is 24px
        let padding = px(24.0);

        point(
            viewport_bounds.origin.x + scroll_offset.x + (content_width - page_width) / 2.0,
            viewport_bounds.origin.y + scroll_offset.y + page_top + padding
        )
    }

    fn recalculate_layout(&mut self) {
        if let Some(doc) = &self.compiled_document {
            let zoom = self.zoom;
            let mut all_tops = Vec::new();
            let mut current_top = px(24.0);
            let mut max_w_pt = 0.0f32;
            let spacing = 24.0 * PT_TO_PX * zoom;

            for page in &doc.pages {
                all_tops.push(current_top);
                let h = px(page.frame.height().to_pt() as f32 * PT_TO_PX * zoom);
                let w = page.frame.width().to_pt() as f32;
                if w > max_w_pt { max_w_pt = w; }
                current_top += h + px(spacing);
            }

            self.page_tops = all_tops;
            self.max_doc_width = px(max_w_pt * PT_TO_PX * zoom);
        }
    }

    pub fn update_document(&mut self, doc: std::sync::Arc<PagedDocument>, version: usize, cx: &mut Context<Self>) {
        self.compiled_document = Some(doc);
        self.last_document_version = Some(version);
        self.recalculate_layout();
        cx.notify();
    }
    pub fn scroll_to_caret(&mut self, page_idx: usize, pos: Point<Pixels>, height: f32, cx: &mut Context<Self>) {
        if page_idx >= self.page_tops.len() {
            return;
        }
        let zoom = self.zoom;
        let page_top = self.page_tops[page_idx];
        let caret_y = page_top + pos.y * zoom;
        let caret_h = px(height * zoom);
        
        let viewport_height = self.scroll_handle.bounds().size.height;
        let scroll_offset = self.scroll_handle.offset();
        let current_scroll_top = -scroll_offset.y;

        if viewport_height <= px(0.0) {
            // Viewport not laid out yet, defer or use a standard estimate
            let target_y = caret_y - px(300.0);
            self.scroll_handle.set_offset(point(px(0.0), -target_y));
            cx.notify();
        } else {
            let margin = viewport_height * 0.1; // 10% margin
            let visible_min = current_scroll_top + margin;
            let visible_max = current_scroll_top + viewport_height - margin - caret_h;

            if caret_y < visible_min || caret_y > visible_max {
                // Out of sight! Center it.
                let target_y = caret_y - viewport_height / 2.0;
                self.scroll_handle.set_offset(point(px(0.0), -target_y));
                cx.notify();
            }
        }
    }

    fn render_typst_document(&mut self, doc: std::sync::Arc<PagedDocument>, caret_visible: bool, cx: &mut Context<Self>) -> Div {
        let zoom = self.zoom;
        let spacing = 24.0 * PT_TO_PX * zoom;
        
        let workspace_handle = self.workspace.upgrade().expect("Workspace dropped");
        let workspace = workspace_handle.read(cx);

        let viewport_size = self.scroll_handle.bounds().size;
        let scroll_offset = self.scroll_handle.offset();
        
        // PERFORMANCE: Add overscan to visibility check to prevent flicker during fast scrolling
        let overscan = px(600.0); 
        let viewport_top = -scroll_offset.y - overscan;
        let viewport_bottom = -scroll_offset.y + viewport_size.height + overscan;

        let selection_range = workspace.selection.as_ref().map(|s| (s.start, s.end));

        let source_paint = workspace.compiled_source.clone().unwrap_or_else(|| workspace.world.source_ref().clone());
        let absolute_cursor = workspace.cursor_relative_pos;

        let num_pages = doc.pages.len();
        if num_pages == 0 { return div(); }

        // PERFORMANCE: Binary search for visible page range to keep element tree size O(Visible) instead of O(Total)
        let first_visible_idx = self.page_tops.partition_point(|&top| {
            // A conservative estimate: if top of page is 1000px above viewport_top, it's definitely not visible.
            // In a real Typst doc, pages are at least several hundred px high.
            top < viewport_top - px(1500.0) 
        });

        let mut last_visible_idx = first_visible_idx;
        while last_visible_idx < num_pages {
            let page_top = self.page_tops.get(last_visible_idx).cloned().unwrap_or(px(0.0));
            if page_top > viewport_bottom {
                break;
            }
            last_visible_idx += 1;
        }
        last_visible_idx = last_visible_idx.min(num_pages);

        // Calculate total document height for correct scrolling
        let total_height = if num_pages > 0 {
            let last_page_idx = num_pages - 1;
            let last_page_top = self.page_tops.get(last_page_idx).cloned().unwrap_or(px(0.0));
            let last_page_height = doc.pages.get(last_page_idx)
                .map(|p| px(p.frame.height().to_pt() as f32 * PT_TO_PX * zoom))
                .unwrap_or(px(0.0));
            last_page_top + last_page_height + px(spacing)
        } else {
            px(0.0)
        };

        div()
            .relative()
            .w_full()
            .h(total_height)
            .children(
                (first_visible_idx..last_visible_idx).filter_map(|i| {
                    let page = doc.pages.get(i)?;
                    let page_width = px(page.frame.width().to_pt() as f32 * PT_TO_PX * zoom);
                    let page_height = px(page.frame.height().to_pt() as f32 * PT_TO_PX * zoom);
                    let page_top = self.page_tops.get(i).cloned().unwrap_or(px(0.0));
                    
                    let source_for_paint = source_paint.clone();
                    let doc_for_canvas = doc.clone();
                    let _doc_for_mouse_down = doc.clone();
                    let _doc_for_mouse_move = doc.clone();
                    let font_cache = self.font_cache.clone();
                    let page_canvas_origins = self.page_canvas_origins.clone();
                    #[cfg(debug_assertions)]
                    let this_show_boxes = self.show_glyph_boxes;

                    Some(div()
                        .absolute()
                        .top(page_top)
                        .w_full()
                        .flex()
                        .justify_center()
                        .child(
                            div()
                                .w(page_width)
                                .h(page_height)
                                .flex_none()
                                .bg(gpui::white())
                                .shadow_xl()
                                .child(
                                    canvas(
                                        move |_, _, _| {},
                                        move |bounds, _prepaint, window, cx| {
                                            // Capture this page's window-space canvas origin so mouse
                                            // handlers can convert ev.position (window space) to
                                            // page-local glyph-box coordinates.
                                            {
                                                let mut origins = page_canvas_origins.lock().unwrap();
                                                if origins.len() <= i {
                                                    origins.resize(i + 1, Point::default());
                                                }
                                                origins[i] = bounds.origin;
                                            }

                                            if let Some(page) = doc_for_canvas.pages.get(i) {
                                                let frame = &page.frame;
                                                #[cfg(debug_assertions)]
                                                let show_boxes = this_show_boxes;
                                                #[cfg(not(debug_assertions))]
                                                let show_boxes = false;

                                                painter::paint_frame(frame, bounds.origin, window, cx, &source_for_paint, selection_range, zoom, show_boxes, &font_cache);

                                                if let Some((_caret_idx, rel_cursor, cursor_h_px)) = absolute_cursor.filter(|(idx, _, _)| *idx == i && caret_visible) {
                                                    let cursor_h = px(cursor_h_px * zoom * 0.9);
                                                    let caret_top = rel_cursor.y * zoom - cursor_h * 0.82;
                                                    let cursor_bounds = Bounds {
                                                        origin: point(bounds.origin.x + rel_cursor.x * zoom, bounds.origin.y + caret_top),
                                                        size: size(px(1.5), cursor_h),
                                                    };
                                                    window.paint_quad(quad(
                                                        cursor_bounds, 
                                                        px(0.0), 
                                                        gpui::rgba(0x3b82f6ff), 
                                                        px(0.0), 
                                                        gpui::transparent_black(), 
                                                        Default::default()
                                                    ));
                                                }
                                            }
                                        },
                                    )
                                    .size_full(),
                                )
                                .on_mouse_down(MouseButton::Left, cx.listener(move |this, ev, window, cx| {
                                    this.handle_mouse_down(ev, window, cx, i);
                                }))
                                .on_mouse_move(cx.listener(move |this, ev, window, cx| {
                                    this.handle_mouse_move(ev, window, cx, i);
                                }))
                                .on_mouse_up(MouseButton::Left, cx.listener(move |this, _, _, cx| this.handle_mouse_up(cx)))
                        ))
                })
            )
    }
}

impl Render for RendererView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let has_active_editor = self.workspace.upgrade()
            .map(|ws| ws.read(cx).active_editor_path.is_some())
            .unwrap_or(false);
        let doc = if has_active_editor {
            self.compiled_document.clone()
        } else {
            None
        };
        let error = self.error_message.clone();

        let viewport_size = self.scroll_handle.bounds().size;
        // Calculate world width using cached max_doc_width
        let content_width = self.max_doc_width.max(viewport_size.width);

        // PERFORMANCE: Drive caret animation only when focused
        let mut is_focused = self.focus_handle.is_focused(_window);
        if !is_focused && let Some(ws_handle) = self.workspace.upgrade() {
            let ws = ws_handle.read(cx);
            if let Some(active_path) = &ws.active_editor_path {
                if let Some(editor) = ws.editors.get(active_path) {
                    is_focused = editor.read(cx).focus_handle(cx).is_focused(_window);
                }
            }
        }
        let now = std::time::Instant::now();
        let millis_since_activity = now.duration_since(self.last_keystroke_at).as_millis();
        
        let caret_visible = if is_focused {
            cx.on_next_frame(_window, |_this: &mut Self, _window, cx| cx.notify());
            
            if millis_since_activity < 500 {
                true // Solid when typing
            } else {
                (millis_since_activity / 500).is_multiple_of(2) // Blink 1Hz
            }        } else {
            true // Always visible when not focused
        };

        let view_entity = cx.entity().clone();
        let view_entity_for_canvas = view_entity.clone();
        let focus_handle = self.focus_handle.clone();
        let inner_view = div()
            .relative() // Required for manual scrollbar anchoring
            .size_full()
            .bg(cx.theme().background)
            .track_focus(&self.focus_handle)
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, _, window, cx| {
                        let handler = gpui::ElementInputHandler::new(bounds, view_entity_for_canvas.clone());
                        window.handle_input(&focus_handle, handler, cx);
                    }
                ).size_full().absolute()
            )
            .on_action(cx.listener(|this, _action: &crate::ui::workspace::OpenCommandPalette, window, cx| {
                this.focus_handle(cx).focus(window);
            }))
            .on_action(cx.listener(|this, _action: &gpui_component::input::Copy, _window, cx| {
                let ws_handle = this.workspace.upgrade().expect("Workspace dropped");
                ws_handle.update(cx, |ws, cx| {
                    if let Some(sel) = &ws.selection {
                        // Normalize: selection may be stored backwards (start > end)
                        let text = ws.world.source_ref().text();
                        let lo = sel.start.min(sel.end).min(text.len());
                        let hi = sel.start.max(sel.end).min(text.len());
                        if lo < hi {
                            // Clamp to valid UTF-8 char boundaries
                            let lo = text.floor_char_boundary(lo);
                            let hi = text.ceil_char_boundary(hi);
                            let selected_text = text[lo..hi].to_string();
                            cx.write_to_clipboard(ClipboardItem::new_string(selected_text));
                        }
                    }
                });
            }))
            .on_action(cx.listener(|this, _action: &gpui_component::input::Cut, window, cx| {
                let ws_handle = this.workspace.upgrade().expect("Workspace dropped");
                ws_handle.update(cx, |ws, cx| {
                    if let Some(sel) = &ws.selection {
                        // Normalize: selection may be stored backwards (start > end)
                        let text = ws.world.source_ref().text();
                        let lo = sel.start.min(sel.end).min(text.len());
                        let hi = sel.start.max(sel.end).min(text.len());
                        if lo < hi {
                            let lo = text.floor_char_boundary(lo);
                            let hi = text.ceil_char_boundary(hi);
                            let selected_text = text[lo..hi].to_string();
                            cx.write_to_clipboard(ClipboardItem::new_string(selected_text));
                            ws.apply_editor_action(crate::core::editor::EditorAction::Edit {
                                range: lo..hi,
                                replacement: "".to_string(),
                                new_cursor: lo,
                                new_selection: None,
                            }, window, cx);
                        }
                    }
                });
            }))
            .on_action(cx.listener(|this, _action: &gpui_component::input::Paste, window, cx| {
                let ws_handle = this.workspace.upgrade().expect("Workspace dropped");
                if let Some(clipboard) = cx.read_from_clipboard() {
                    if let Some(paste_text) = clipboard.text() {
                        ws_handle.update(cx, |ws, cx| {
                            let text = ws.world.source_ref().text();
                            let (raw_start, raw_end) = if let Some(sel) = &ws.selection {
                                (sel.start.min(sel.end), sel.start.max(sel.end))
                            } else {
                                (ws.cursor_offset, ws.cursor_offset)
                            };
                            let lo = text.floor_char_boundary(raw_start.min(text.len()));
                            let hi = text.ceil_char_boundary(raw_end.min(text.len()));
                            ws.apply_editor_action(crate::core::editor::EditorAction::Edit {
                                range: lo..hi,
                                replacement: paste_text.clone(),
                                new_cursor: lo + paste_text.len(),
                                new_selection: None,
                            }, window, cx);
                        });
                    }
                }
            }))
            .on_action(cx.listener(|this, _action: &gpui_component::input::Undo, window, cx| {
                let ws_handle = this.workspace.upgrade().expect("Workspace dropped");
                ws_handle.update(cx, |ws, cx| {
                    ws.undo(window, cx);
                });
            }))
            .on_action(cx.listener(|this, _action: &gpui_component::input::Redo, window, cx| {
                let ws_handle = this.workspace.upgrade().expect("Workspace dropped");
                ws_handle.update(cx, |ws, cx| {
                    ws.redo(window, cx);
                });
            }))
            .on_action(cx.listener(|this, _action: &gpui_component::input::MovePageUp, _window, cx| {
                let viewport_height = this.scroll_handle.bounds().size.height;
                let mut offset = this.scroll_handle.offset();
                offset.y += viewport_height * 0.9;
                this.scroll_handle.set_offset(offset);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _action: &gpui_component::input::MovePageDown, _window, cx| {
                let viewport_height = this.scroll_handle.bounds().size.height;
                let mut offset = this.scroll_handle.offset();
                offset.y -= viewport_height * 0.9;
                this.scroll_handle.set_offset(offset);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, window, cx| {
                this.handle_key_down(ev, window, cx);
            }))
            .child(
                div()
                    .id("wysiwyg-scroll")
                    .track_scroll(&self.scroll_handle)
                    .overflow_scroll()
                    .size_full()
                    .on_scroll_wheel(cx.listener(|this, ev: &ScrollWheelEvent, window, cx| {
                        if ev.modifiers.control || ev.modifiers.platform {
                            let delta_y = ev.delta.pixel_delta(px(20.0)).y;
                            if delta_y != px(0.0) {
                                let factor = if delta_y > px(0.0) { 1.05 } else { 1.0 / 1.05 };
                                let new_zoom = (this.zoom * factor).clamp(0.2, 5.0);
                                if new_zoom != this.zoom {
                                    this.zoom = new_zoom;
                                    let zoom = this.zoom;
                                    this.zoom_slider_state.update(cx, |state, cx| {
                                        state.set_value(zoom, window, cx);
                                    });
                                    this.recalculate_layout();
                                    cx.notify();
                                }
                            }
                        } else {
                            cx.propagate();
                        }
                    }))
                    .child(
                        v_flex()
                            .min_w(content_width)
                            .min_h_full()
                            .p_6()
                            .child(if let Some(d) = doc.clone() {
                                self.render_typst_document(d, caret_visible, cx)
                            } else {
                                h_flex()
                                    .w_full()
                                    .h(px(400.0))
                                    .justify_center()
                                    .items_center()
                                    .text_color(rgb(0x64748b))
                                    .text_sm()
                                    .child("Open a Typst file to preview")
                            })
                    )
            )
            .child(
                if let Some(err) = error {
                    div()
                        .absolute()
                        .bottom_4()
                        .right_4()
                        .max_w_80()
                        .bg(rgb(0x450a0a))
                        .text_color(rgb(0xfecaca))
                        .text_xs()
                        .p_2()
                        .rounded_md()
                        .border_1()
                        .border_color(rgb(0x991b1b))
                        .child(err)
                } else {
                    div()
                }
            )
            .child(div().absolute().right_0().top_0().bottom_0().child(gpui_component::scroll::Scrollbar::vertical(&self.scroll_handle)))
            .child(div().absolute().left_0().bottom_0().right_0().child(gpui_component::scroll::Scrollbar::horizontal(&self.scroll_handle)));

        div()
            .size_full()
            .cursor_default()
            .track_focus(&self.focus_handle)
            .key_context("Renderer")
            .on_mouse_down(MouseButton::Left, {
                let focus_handle = self.focus_handle.clone();
                move |_, window, _| {
                    focus_handle.focus(window);
                }
            })
            .context_menu({
                let view_entity = view_entity.clone();
                let workspace = self.workspace.clone();
                move |menu, _, cx| {
                    let has_selection = if let Some(ws) = workspace.upgrade() {
                        ws.read(cx).selection.map(|s| !s.is_empty()).unwrap_or(false)
                    } else {
                        false
                    };
                    let has_paste = cx.read_from_clipboard().is_some();

                    let mut menu = menu;
                    if let Some(ws) = workspace.upgrade() {
                        let active_editor = ws.read(cx).active_editor_path.as_ref()
                            .and_then(|path| ws.read(cx).editors.get(path).cloned());
                        
                        if let Some(editor) = active_editor {
                            let editor_focus = editor.read(cx).input.read(cx).focus_handle(cx);
                            menu = menu.action_context(editor_focus);
                        }
                    }

                    menu
                        .menu_with_enable(
                            "Cut",
                            Box::new(gpui_component::input::Cut),
                            has_selection,
                        )
                        .menu_with_enable(
                            "Copy",
                            Box::new(gpui_component::input::Copy),
                            has_selection,
                        )
                        .menu_with_enable(
                            "Paste",
                            Box::new(gpui_component::input::Paste),
                            has_paste,
                        )
                        .separator()
                        .item(PopupMenuItem::new("Export PDF").on_click({
                            let view_entity = view_entity.clone();
                            let workspace = workspace.clone();
                            move |_, _, cx| {
                                view_entity.update(cx, |this, cx| {
                                    if let Some(doc) = &this.compiled_document {
                                        if let Ok(pdf_bytes) = typst_pdf::pdf(doc, &typst_pdf::PdfOptions::default()) {
                                            if let Some(ws_handle) = workspace.upgrade() {
                                                let root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
                                                let pdf_path = root.join("output.pdf");
                                                let _ = std::fs::write(&pdf_path, pdf_bytes);
                                                ws_handle.update(cx, |ws, _cx| {
                                                    ws.log("INFO", format!("Exported PDF to {:?}", pdf_path));
                                                });
                                            }
                                        }
                                    }
                                });
                            }
                        }))
                }
            })
            .child(inner_view)
    }
}

impl Focusable for RendererView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl gpui_component::dock::Panel for RendererView {
    fn panel_name(&self) -> &'static str {
        "Renderer"
    }

    fn title(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        "Preview"
    }

    fn title_suffix(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<impl IntoElement> {
        let zoom = self.zoom;
        let zoom_slider_state = self.zoom_slider_state.clone();
        
        Some(
            h_flex()
                .gap_2()
                .items_center()
                .child(
                    Button::new("zoom-out")
                        .icon(IconName::Minus)
                        .xsmall()
                        .ghost()
                        .on_click(cx.listener(|this, _, window, cx| {
                            let new_zoom = (this.zoom / 1.1).clamp(0.2, 5.0);
                            this.zoom = new_zoom;
                            this.zoom_slider_state.update(cx, |state, cx| state.set_value(new_zoom, window, cx));
                            this.recalculate_layout();
                            cx.notify();
                        }))
                )
                .child(
                    div()
                        .w(px(100.0))
                        .child(gpui_component::slider::Slider::new(&zoom_slider_state))
                )
                .child(
                    Button::new("zoom-in")
                        .icon(IconName::Plus)
                        .xsmall()
                        .ghost()
                        .on_click(cx.listener(|this, _, window, cx| {
                            let new_zoom = (this.zoom * 1.1).clamp(0.2, 5.0);
                            this.zoom = new_zoom;
                            this.zoom_slider_state.update(cx, |state, cx| state.set_value(new_zoom, window, cx));
                            this.recalculate_layout();
                            cx.notify();
                        }))
                )
                .child(
                    div()
                        .w_12()
                        .flex()
                        .justify_end()
                        .child(
                            div()
                                .text_xs()
                                .child(format!("{}%", (zoom * 100.0) as i32))
                        )
                )
                .child(
                    Button::new("reset-zoom")
                        .icon(IconName::Undo)
                        .xsmall()
                        .ghost()
                        .tooltip("Reset Zoom")
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.zoom = 1.0;
                            this.zoom_slider_state.update(cx, |state, cx| state.set_value(1.0, window, cx));
                            this.recalculate_layout();
                            cx.notify();
                        }))
                )
        )
    }

    fn closable(&self, _cx: &App) -> bool {
        false
    }

    fn visible(&self, cx: &App) -> bool {
        self.workspace.upgrade().map(|ws| ws.read(cx).config.preview_panel_visible).unwrap_or(true)
    }
}

impl EventEmitter<gpui_component::dock::PanelEvent> for RendererView {}
