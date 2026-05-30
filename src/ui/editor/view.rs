use crate::ui::workspace::EditorWorkspace;
use gpui::*;
use gpui_component::input::InputState;
use lsp_types::*;
use std::ops::Range;
use std::str::FromStr;
use typst::World;
use typst::syntax::FileId;

pub struct SourceEditorView {
    _workspace: WeakEntity<EditorWorkspace>,
    pub input: Entity<InputState>,
    lsp_client: Option<std::sync::Arc<crate::core::lsp::LspClient>>,
    pub uri: Uri,
    pub window_handle: AnyWindowHandle,

    // Performance snapshots to avoid redundant work
    last_text: gpui_component::Rope,
    last_cursor: usize,
    last_selection: Option<Range<usize>>,
    lsp_sync_task: Option<Task<()>>,
    workspace_sync_task: Option<Task<()>>,

    // Dirty tracking
    pub saved_text: gpui_component::Rope,
    pub is_dirty: bool,
    last_synced_text: String,
}

impl SourceEditorView {
    pub fn new(
        workspace: WeakEntity<EditorWorkspace>,
        lsp_client: Option<std::sync::Arc<crate::core::lsp::LspClient>>,
        window_handle: AnyWindowHandle,
        window: &mut Window,
        cx: &mut Context<Self>,
        path: std::path::PathBuf,
        initial_text: String,
    ) -> Self {
        let initial_text_rope = gpui_component::Rope::from(initial_text.clone());

        // Standardize URI for LSP using the correct file path
        let uri_str = format!("file://{}", path.to_string_lossy());
        let uri = Uri::from_str(&uri_str).unwrap();

        let input = cx.new(|cx| {
            let mut state = InputState::new(window, cx)
                .code_editor("typst")
                .line_number(true)
                .multi_line(true)
                .default_value(initial_text.clone());

            if let Some(client) = lsp_client.clone() {
                use std::rc::Rc;
                let provider = Rc::new(crate::ui::editor::lsp::TypstLspProvider::new(
                    client,
                    uri.clone(),
                ));
                state.lsp.completion_provider = Some(provider);
            }
            state
        });

        if let Some(client) = lsp_client.clone() {
            let _ = client.did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "typst".to_string(),
                    version: 0,
                    text: initial_text.clone(),
                },
            });
        }

        // Sync changes back to workspace
        cx.observe(&input, {
            let workspace = workspace.clone();
            let lsp_client = lsp_client.clone();
            move |this, input, cx| {
                let window_handle = this.window_handle;
                let state = input.read(cx);
                let cursor = state.cursor();
                let selection = state.selected_range;

                // PERFORMANCE: Check if content actually changed before expensive to_string()
                let content_changed = state.text() != &this.last_text;
                let cursor_moved = cursor != this.last_cursor;

                let selection_range = if selection.is_empty() {
                    None
                } else {
                    Some(selection.start..selection.end)
                };
                let selection_changed = selection_range != this.last_selection;

                if !content_changed && !cursor_moved && !selection_changed {
                    return;
                }

                // PERFORMANCE: Clone the Rope (cheap)
                let text_rope = state.text().clone();
                this.last_text = text_rope.clone();
                this.last_cursor = cursor;
                this.last_selection = selection_range.clone();

                if content_changed {
                    let is_dirty = state.text() != &this.saved_text;
                    if is_dirty != this.is_dirty {
                        this.is_dirty = is_dirty;
                        cx.notify();
                    }
                }

                if content_changed && lsp_client.is_some() {
                    // DEBOUNCED LSP SYNC
                    this.lsp_sync_task = None;
                    
                    let new_text = text_rope.clone();
                    let uri_clone = this.uri.clone();
                    
                    this.lsp_sync_task = Some(cx.spawn(move |this_weak: WeakEntity<SourceEditorView>, cx: &mut AsyncApp| {
                        let mut cx = cx.clone();
                        async move {
                            cx.background_executor()
                                .timer(std::time::Duration::from_millis(50))
                                .await;
                            
                            this_weak.update(&mut cx, |this, _cx| {
                                let new_text_rope = new_text.clone();
                                let old_text = &this.last_synced_text;

                                if !rope_eq_str(&new_text_rope, old_text) {
                                    let (common_prefix, common_suffix) =
                                        find_common_prefix_suffix(old_text, &new_text_rope);

                                    let range = common_prefix..(old_text.len() - common_suffix);
                                    let replacement = new_text_rope
                                        .slice(common_prefix..(new_text_rope.len() - common_suffix))
                                        .to_string();

                                    let start_position = byte_offset_to_lsp_position(old_text, range.start);
                                    let end_position = byte_offset_to_lsp_position(old_text, range.end);

                                    let lsp_range = lsp_types::Range {
                                        start: start_position,
                                        end: end_position,
                                    };

                                    if let Some(client) = &this.lsp_client {
                                        let _ = client.did_change(DidChangeTextDocumentParams {
                                            text_document: VersionedTextDocumentIdentifier {
                                                uri: uri_clone,
                                                version: 0,
                                            },
                                            content_changes: vec![TextDocumentContentChangeEvent {
                                                range: Some(lsp_range),
                                                range_length: None,
                                                text: replacement,
                                            }],
                                        });
                                    }

                                    this.last_synced_text = new_text_rope.to_string();
                                }
                            }).ok();
                        }
                    }));
                }

                // ── Synchronous Workspace State Update (Instantaneous) ───────────────────
                if let Some(ws_handle) = workspace.upgrade() {
                    let uri_str = this.uri.to_string();
                    let url = url::Url::from_str(&uri_str).unwrap();

                    cx.update_window(window_handle, |_, window, cx| {
                        ws_handle.update(cx, |ws, cx| {
                            let path = url.to_file_path().unwrap_or_else(|_| {
                                std::path::PathBuf::from(url.path().to_string())
                            });

                            // If this editor is focused, it's the active one
                            let is_focused = input.read(cx).focus_handle(cx).is_focused(window);

                            if is_focused {
                                ws.active_editor_path = Some(path.clone());
                                ws.cursor_offset = cursor;
                                if !ws.is_dragging {
                                    ws.selection = selection_range.map(|r| {
                                        gpui_component::input::Selection::new(r.start, r.end)
                                    });
                                }
                            }

                            // Always update cursor metadata and refresh caret position for active workspace files (instantaneous)
                            if ws.active_editor_path.as_ref() == Some(&path) {
                                ws.update_cursor_node_info(cx);
                                ws.refresh_caret_location(cx, !content_changed);
                            }
                            cx.notify();
                        });
                    })
                    .ok();
                }

                // ── Debounced Workspace Text Sync (State-of-the-Art, 60fps+ typing) ─────
                if content_changed {
                    this.workspace_sync_task = None;
                    
                    let new_text = text_rope.clone();
                    let uri_clone = this.uri.clone();
                    let workspace_clone = workspace.clone();
                    let cursor_clone = cursor;
                    let selection_clone = if selection.is_empty() {
                        None
                    } else {
                        Some(selection.start..selection.end)
                    };
                    
                    this.workspace_sync_task = Some(cx.spawn(move |this_weak: WeakEntity<SourceEditorView>, cx: &mut AsyncApp| {
                        let mut cx = cx.clone();
                        async move {
                            cx.background_executor()
                                .timer(std::time::Duration::from_millis(50))
                                .await;
                            
                            this_weak.update(&mut cx, |this, cx| {
                                if let Some(ws_handle) = workspace_clone.upgrade() {
                                    cx.update_window(this.window_handle, |_, window, cx| {
                                        ws_handle.update(cx, |ws, cx| {
                                            let uri_str = uri_clone.to_string();
                                            let url = url::Url::from_str(&uri_str).unwrap();
                                            let path = url.to_file_path().unwrap_or_else(|_| {
                                                std::path::PathBuf::from(url.path().to_string())
                                            });

                                            let root = ws
                                                .world
                                                .root_path
                                                .clone()
                                                .unwrap_or_else(|| std::path::PathBuf::from("."));
                                            let vpath = typst::syntax::VirtualPath::within_root(&path, &root)
                                                .unwrap_or_else(|| {
                                                    typst::syntax::VirtualPath::new(path.file_name().unwrap())
                                                });
                                            let id = FileId::new(None, vpath);

                                            if let Ok(s) = ws.world.source(id) {
                                                let old_text = s.text();
                                                if !rope_eq_str(&new_text, old_text) {
                                                    let (common_prefix, common_suffix) =
                                                        find_common_prefix_suffix(old_text, &new_text);

                                                    let range = common_prefix..(old_text.len() - common_suffix);
                                                    let replacement = new_text
                                                        .slice(common_prefix..(new_text.len() - common_suffix))
                                                        .to_string();

                                                    ws.apply_editor_action_from_editor(
                                                        &url,
                                                        crate::core::editor::EditorAction::Edit {
                                                            range,
                                                            replacement,
                                                            new_cursor: cursor_clone,
                                                            new_selection: selection_clone,
                                                        },
                                                        window,
                                                        cx,
                                                    );
                                                }
                                            }
                                        });
                                    }).ok();
                                }
                            }).ok();
                        }
                    }));
                }
            }
        })
        .detach();

        Self {
            _workspace: workspace,
            input,
            lsp_client,
            uri,
            window_handle,
            last_text: initial_text_rope.clone(),
            last_cursor: 0,
            last_selection: None,
            lsp_sync_task: None,
            workspace_sync_task: None,
            saved_text: initial_text_rope,
            is_dirty: false,
            last_synced_text: initial_text.clone(),
        }
    }

    pub fn open_file(
        &mut self,
        uri: Uri,
        content: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Clear pending sync task
        self.lsp_sync_task = None;
        self.workspace_sync_task = None;

        // did_close the old file if LSP is active
        if let Some(client) = &self.lsp_client {
            let _ = client.did_close(DidCloseTextDocumentParams {
                text_document: TextDocumentIdentifier {
                    uri: self.uri.clone(),
                },
            });
            // did_open the new file
            let _ = client.did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "typst".to_string(),
                    version: 0,
                    text: content.clone(),
                },
            });
        }

        self.uri = uri.clone();
        self.last_text = gpui_component::Rope::from(content.clone());
        self.last_cursor = 0;
        self.last_selection = None;

        self.input.update(cx, |input, cx| {
            input.set_value(content, window, cx);

            if let Some(client) = &self.lsp_client {
                use std::rc::Rc;
                let provider = Rc::new(crate::ui::editor::lsp::TypstLspProvider::new(
                    client.clone(),
                    uri.clone(),
                ));
                input.lsp.completion_provider = Some(provider.clone());
                input.lsp.hover_provider = Some(provider);
            }
        });
        cx.notify();
    }

    pub fn scroll_to_offset(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.input.update(cx, |input, cx| {
            input.scroll_to(offset, None, cx);
        });
    }
}

impl Render for SourceEditorView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .track_focus(&self.focus_handle(_cx))
            .child(
                gpui_component::input::Input::new(&self.input)
                    .size_full()
                    .rounded_none(),
            )
    }
}

impl Focusable for SourceEditorView {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.input.read(cx).focus_handle(cx)
    }
}

impl gpui_component::dock::Panel for SourceEditorView {
    fn panel_name(&self) -> &'static str {
        "SourceEditor"
    }

    fn tab_name(&self, _cx: &App) -> Option<SharedString> {
        let path = url::Url::parse(&self.uri.to_string())
            .ok()
            .and_then(|u| u.to_file_path().ok())
            .unwrap_or_else(|| std::path::PathBuf::from(self.uri.path().to_string()));
        let mut name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Source".to_string());
        if self.is_dirty {
            name.push_str(" *");
        }
        Some(name.into())
    }

    fn title(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let path = url::Url::parse(&self.uri.to_string())
            .ok()
            .and_then(|u| u.to_file_path().ok())
            .unwrap_or_else(|| std::path::PathBuf::from(self.uri.path().to_string()));
        let mut name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Source Editor".to_string());
        if self.is_dirty {
            name.push_str(" *");
        }
        name
    }

    fn closable(&self, _cx: &App) -> bool {
        true
    }

    fn visible(&self, cx: &App) -> bool {
        self._workspace
            .upgrade()
            .map(|ws| ws.read(cx).config.source_code_visible)
            .unwrap_or(true)
    }

    fn on_removed(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(ws_handle) = self._workspace.upgrade() {
            let uri_str = self.uri.to_string();
            let url = url::Url::from_str(&uri_str).unwrap();
            let path = url
                .to_file_path()
                .unwrap_or_else(|_| std::path::PathBuf::from(url.path().to_string()));
            ws_handle.update(cx, |ws, cx| {
                ws.editors.remove(&path);
                if ws.active_editor_path.as_ref() == Some(&path) {
                    ws.active_editor_path = None;
                }
                cx.notify();
            });
        }
    }
}

impl EventEmitter<gpui_component::dock::PanelEvent> for SourceEditorView {}

fn rope_eq_str(rope: &gpui_component::Rope, s: &str) -> bool {
    if rope.len() != s.len() {
        return false;
    }
    let mut bytes = s.as_bytes();
    for chunk in rope.chunks() {
        let chunk_bytes = chunk.as_bytes();
        if !bytes.starts_with(chunk_bytes) {
            return false;
        }
        bytes = &bytes[chunk_bytes.len()..];
    }
    true
}

pub(crate) fn find_common_prefix_suffix(old: &str, new: &gpui_component::Rope) -> (usize, usize) {
    let old_bytes = old.as_bytes();
    let old_len = old_bytes.len();
    let new_len = new.len();

    let mut common_prefix = 0;
    let mut old_iter = old_bytes.iter();
    let mut new_iter = new.bytes();

    while let (Some(&o), Some(n)) = (old_iter.next(), new_iter.next()) {
        if o != n {
            break;
        }
        common_prefix += 1;
    }

    let mut common_suffix = 0;
    let max_suffix = (old_len - common_prefix).min(new_len - common_prefix);

    if max_suffix > 0 {
        let suffix_slice = new.slice(new_len - max_suffix..new_len);
        let mut chunks = Vec::new();
        for chunk in suffix_slice.chunks() {
            chunks.push(chunk);
        }

        let mut old_bytes_rev = old_bytes[old_len - max_suffix..old_len].iter().rev();

        'outer: for chunk in chunks.iter().rev() {
            let chunk_bytes = chunk.as_bytes();
            for &n in chunk_bytes.iter().rev() {
                if let Some(&o) = old_bytes_rev.next() {
                    if o != n {
                        break 'outer;
                    }
                    common_suffix += 1;
                } else {
                    break 'outer;
                }
            }
        }
    }

    (common_prefix, common_suffix)
}

fn byte_offset_to_lsp_position(text: &str, offset: usize) -> lsp_types::Position {
    let mut line = 0;
    let mut character = 0;

    for (byte_idx, c) in text.char_indices() {
        if byte_idx >= offset {
            break;
        }
        if c == '\n' {
            line += 1;
            character = 0;
        } else {
            character += c.len_utf16() as u32;
        }
    }

    lsp_types::Position { line, character }
}



