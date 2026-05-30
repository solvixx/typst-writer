use gpui::*;
use url::Url;
use std::str::FromStr;
use std::time::Duration;
use std::sync::Arc;
use std::path::PathBuf;
use typst::World;
use typst::layout::PagedDocument;
use typst::syntax::{Source, LinkedNode, SyntaxKind, FileId};
use typst::diag::SourceDiagnostic;
use typst::ecow::EcoVec;

use crate::core::compiler::SimpleWorld;
use crate::core::editor::{EditorContext, map_offset_between_texts};
use crate::ui::renderer::geometry;
use gpui_component::{RopeExt, WindowExt};
use gpui_component::ActiveTheme;

#[derive(Clone, serde::Deserialize, PartialEq, gpui::Action)]
pub struct OpenCommandPalette;

#[derive(Clone, serde::Deserialize, PartialEq, gpui::Action)]
pub struct SaveFile;

#[derive(Clone, serde::Deserialize, PartialEq, gpui::Action)]
pub struct SaveFileAs;

#[derive(Clone, serde::Deserialize, PartialEq, gpui::Action)]
pub struct NewDocument;

#[derive(Clone, serde::Deserialize, PartialEq, gpui::Action)]
pub struct OpenPreferences;

/// EditorWorkspace is the central state management view for the application.
/// It coordinates the Typst compiler, multiple editors, the render panel,
/// and handles all user actions.
pub struct EditorWorkspace {
    /// The Typst world instance for compilation.
    pub(crate) world: SimpleWorld,
    /// Manages background compilation and document versions.
    pub compiler: crate::core::compiler::CompilerManager,
    /// Manages project-level state like working directory and main file.
    pub project: crate::core::project::ProjectManager,
    
    /// The current successfully compiled document.
    pub(crate) compiled_document: Option<Arc<PagedDocument>>,
    /// A snapshot of the source used for the current compiled document.
    pub(crate) compiled_source: Option<Source>,
    /// Error message from the last compilation, if any.
    pub(crate) error_message: Option<String>,
    /// Current byte offset of the cursor in the active source file.
    pub(crate) cursor_offset: usize,
    /// Current visual position of the cursor in the render panel.
    /// Format: (page_index, point_relative_to_page, caret_height)
    pub(crate) cursor_relative_pos: Option<(usize, Point<Pixels>, f32)>,
    /// Information about the currently selected AST node.
    pub(crate) selected_node_info: Option<String>,
    
    /// The semantic context of the current editor (Markup, Math, or Code).
    pub(crate) editor_context: EditorContext,
    /// Title of the current context for UI display.
    pub(crate) context_title: String,
    /// Description of the current context for UI display.
    pub(crate) context_desc: String,
    /// Application configuration.
    pub(crate) config: crate::core::config::AppConfig,
    /// Focus handle for the workspace.
    pub focus_handle: FocusHandle,
    /// Handle to the current window.
    pub window_handle: AnyWindowHandle,

    /// Current text selection range.
    pub(crate) selection: Option<gpui_component::input::Selection>,
    /// Whether the user is currently dragging the mouse to select text.
    pub(crate) is_dragging: bool,

    /// Handle to the background compilation task.
    pub(crate) background_task: Option<gpui::Task<()>>,

    /// Title bar component.
    pub(crate) title_bar: Entity<crate::ui::components::title_bar::TitleBar>,
    /// Ribbon component (toolbar).
    pub(crate) ribbon: Entity<crate::ui::components::ribbon::Ribbon>,
    /// Open editors, keyed by their filesystem path.
    pub(crate) editors: std::collections::HashMap<std::path::PathBuf, Entity<crate::ui::editor::SourceEditorView>>,
    /// Path of the currently active editor.
    pub(crate) active_editor_path: Option<std::path::PathBuf>,
    /// The render panel sub-view.
    pub(crate) _renderer: Entity<crate::ui::renderer::RendererView>,
    /// The main dock area for flexible layout.
    pub(crate) dock_area: Entity<gpui_component::dock::DockArea>,
    /// File tree component for project navigation.
    pub(crate) file_tree: Option<Entity<crate::ui::components::file_tree::FileTree>>,

    /// Application logs.
    pub(crate) logs: Vec<(String, String, String)>,
    /// Whether the log panel is shown.
    pub(crate) _show_logs: bool,

    /// Undo/redo state manager.
    pub(crate) undo_manager: crate::core::editor::UndoManager,

    /// Command palette component.
    pub(crate) command_palette: Option<Entity<crate::ui::components::command_palette::CommandPalette>>,
    /// LSP client for advanced editor features.
    pub(crate) _lsp_client: Option<std::sync::Arc<crate::core::lsp::LspClient>>,
    /// Current version of the document (increments on every compile).
    pub(crate) document_version: usize,

    #[cfg(debug_assertions)]
    /// Whether to show glyph bounding boxes (debug only).
    pub(crate) show_glyph_boxes: bool,
}

impl EditorWorkspace {
    pub fn new(window: &mut Window, cx: &mut Context<Self>, _initial_text: &str) -> Self {
        let config = crate::core::config::ConfigManager::load();
        
        let root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let main_path = root.join("main.typ");
        
        let lsp_client = crate::core::lsp::LspClient::new("tinymist", &[] as &[String], cx).ok();
        
        let workspace_weak = cx.weak_entity();
        let window_handle = window.handle_any();

        // Managers
        let world = SimpleWorld::with_root(root.clone(), main_path.clone());
        let project_manager = crate::core::project::ProjectManager::new(root.clone());
        let compiler_manager = crate::core::compiler::CompilerManager::new();

        // Components
        let title_bar = cx.new(|cx| crate::ui::components::title_bar::TitleBar::new(workspace_weak.clone(), cx));
        let ribbon = cx.new(|_cx| crate::ui::components::ribbon::Ribbon::new(workspace_weak.clone()));

        let editors = std::collections::HashMap::new();

        let renderer = cx.new(|cx| crate::ui::renderer::RendererView::new(workspace_weak.clone(), cx));

        let file_tree_entity = cx.new(|cx| crate::ui::components::file_tree::FileTree::new(workspace_weak.clone(), root.clone(), window, cx));

        let dock_area = {
            let file_tree_for_dock = file_tree_entity.clone();
            let renderer_for_dock = renderer.clone();
            let sidebar_visible = config.sidebar_visible;
            let log_panel_visible = config.log_panel_visible;
            
            cx.new(move |cx| {
                use gpui_component::dock::*;
                let mut dock_area = DockArea::new("main-dock", None, window, cx)
                    .panel_style(PanelStyle::TabBar);
                
                let center_tabs = DockItem::tab(renderer_for_dock, &cx.entity().downgrade(), window, cx);
                let center = DockItem::h_split(
                    vec![center_tabs],
                    &cx.entity().downgrade(),
                    window,
                    cx,
                );
                
                dock_area.set_center(center, window, cx);

                dock_area.set_left_dock(
                    DockItem::tabs(
                        vec![
                            Arc::new(file_tree_for_dock),
                        ],
                        &cx.entity().downgrade(),
                        window,
                        cx,
                    ),
                    Some(px(250.0)),
                    sidebar_visible,
                    window,
                    cx,
                );

                // Bottom Dock: Logs
                let log_panel = cx.new(|_cx| crate::ui::components::log_panel::LogPanel::new(workspace_weak.clone(), _cx));
                dock_area.set_bottom_dock(
                    DockItem::panel(Arc::new(log_panel)),
                    Some(px(200.0)),
                    log_panel_visible,
                    window,
                    cx,
                );

                dock_area
            })
        };

        let app = Self {
            world,
            compiler: compiler_manager,
            project: project_manager,
            compiled_document: None,
            compiled_source: None,
            error_message: None,
            cursor_offset: 0,
            cursor_relative_pos: None,
            selected_node_info: None,
            editor_context: EditorContext::Markup,
            context_title: "Home".to_string(),
            context_desc: "Formatting and text styles".to_string(),
            config: config.clone(),
            focus_handle: cx.focus_handle(),
            window_handle,
            selection: None,
            is_dragging: false,
            background_task: None,
            title_bar,
            ribbon,
            editors,
            active_editor_path: None,
            _renderer: renderer,
            dock_area,
            file_tree: Some(file_tree_entity),
            logs: Vec::new(),
            _show_logs: config.log_panel_visible,
            undo_manager: crate::core::editor::UndoManager::new(200),
            command_palette: None,
            _lsp_client: lsp_client,
            document_version: 0,
            #[cfg(debug_assertions)]
            show_glyph_boxes: false,
        };

        cx.bind_keys([
            gpui::KeyBinding::new("ctrl-s", SaveFile, None),
            gpui::KeyBinding::new("cmd-s", SaveFile, None),
            gpui::KeyBinding::new("ctrl-shift-s", SaveFileAs, None),
            gpui::KeyBinding::new("cmd-shift-s", SaveFileAs, None),
            gpui::KeyBinding::new("ctrl-n", NewDocument, None),
            gpui::KeyBinding::new("cmd-n", NewDocument, None),
        ]);

        app
    }

    pub fn schedule_background_compile(&mut self, cx: &mut Context<Self>) {
        if self.compiler.is_compiling {
            self.compiler.needs_recompile = true;
            return;
        }

        self.background_task = None;
        let this_weak = cx.weak_entity();

        self.background_task = Some(cx.spawn(move |_, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                cx.background_executor().timer(Duration::from_millis(10)).await;
                this_weak.update(&mut cx, |this, cx| {
                    this.compile(cx);
                }).ok();
            }
        }));
    }

    pub fn compile(&mut self, cx: &mut Context<Self>) {
        if self.compiler.is_compiling {
            self.compiler.needs_recompile = true;
            return;
        }

        self.compiler.is_compiling = true;
        self.compiler.needs_recompile = false;

        let world_clone = self.world.clone();
        let this_weak = cx.weak_entity();

        cx.spawn(move |_, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let start = std::time::Instant::now();
                let result = cx.background_executor().spawn(async move {
                    typst::compile::<PagedDocument>(&world_clone)
                }).await;
                let _duration = start.elapsed();

                this_weak.update(&mut cx, |this, cx| {
                    this.compiler.is_compiling = false;
                    
                    match result.output {
                        Ok(doc) => {
                            this.compiled_document = Some(Arc::new(doc.clone()));
                            this.compiled_source = Some(this.world.main_source.clone());
                            this.error_message = None;
                            this.document_version += 1;
                            
                            let text = this.world.main_source.text();
                            this.compiler.update_structural_metadata(text);

                            // Sync to renderer
                            let doc_arc = Arc::new(doc);
                            let version = this.document_version;
                            this._renderer.update(cx, |renderer, cx| {
                                renderer.update_document(doc_arc, version, cx);
                            });

                            this.refresh_caret_location(cx, false);
                            this.sync_diagnostics(&result.warnings, cx);
                        }
                        Err(diags) => {
                            if !diags.is_empty() {
                                this.error_message = Some(diags[0].message.to_string());
                            }
                            this.sync_diagnostics(&diags, cx);
                        }
                    }

                    if this.compiler.needs_recompile {
                        this.schedule_background_compile(cx);
                    }
                    cx.notify();
                }).ok();
            }
        }).detach();
    }

    fn sync_diagnostics(&mut self, diags: &EcoVec<SourceDiagnostic>, cx: &mut Context<Self>) {
        use gpui_component::highlighter::{Diagnostic, DiagnosticSeverity};

        let mut file_diags: std::collections::HashMap<FileId, Vec<gpui_component::highlighter::Diagnostic>> = std::collections::HashMap::new();

        for diag in diags {
            let id = if let Some(id) = diag.span.id() { id } else { continue };
            let source = if let Ok(s) = self.world.source(id) { s } else { continue };
            
            let range = source.range(diag.span).unwrap_or(0..0);
            let lines = source.lines();
            let pos_start = lines.byte_to_line(range.start).unwrap_or(0);
            let col_start = lines.byte_to_column(range.start).unwrap_or(0);
            let pos_end = lines.byte_to_line(range.end).unwrap_or(0);
            let col_end = lines.byte_to_column(range.end).unwrap_or(0);

            let severity = match diag.severity {
                typst::diag::Severity::Error => DiagnosticSeverity::Error,
                typst::diag::Severity::Warning => DiagnosticSeverity::Warning,
            };

            let message = diag.message.to_string();
            let start = gpui_component::input::Position::new(pos_start as u32, col_start as u32);
            let end = gpui_component::input::Position::new(pos_end as u32, col_end as u32);
            
            file_diags.entry(id).or_default().push(
                Diagnostic::new(start..end, message)
                    .with_severity(severity)
                    .with_source("Typst Compiler")
            );
        }

        // Clear all editor diagnostics first
        for editor in self.editors.values() {
            editor.update(cx, |view, cx| {
                view.input.update(cx, |state, cx| {
                    if let Some(ds) = state.diagnostics_mut() {
                        ds.clear();
                    }
                    cx.notify();
                });
            });
        }

        // Distribute new diagnostics
        for (id, diags) in file_diags {
            let path = if let Some(root) = &self.world.root_path {
                id.vpath().resolve(root)
            } else {
                None
            };
            
            if let Some(path) = path {
                if let Some(editor) = self.editors.get(&path) {
                    editor.update(cx, |view, cx| {
                        view.input.update(cx, |state, cx| {
                            if let Some(ds) = state.diagnostics_mut() {
                                ds.extend(diags);
                            }
                            cx.notify();
                        });
                    });
                }
            }
        }
    }

    pub fn update_panel_visibility(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let sidebar_visible = self.config.sidebar_visible;
        let log_panel_visible = self.config.log_panel_visible;
        
        self.dock_area.update(cx, |dock, cx| {
            dock.set_dock_open(gpui_component::dock::DockPlacement::Left, sidebar_visible, window, cx);
            dock.set_dock_open(gpui_component::dock::DockPlacement::Bottom, log_panel_visible, window, cx);
        });
        self._show_logs = log_panel_visible;
    }

    pub fn log(&mut self, level: &str, message: String) {
        let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
        self.logs.push((timestamp, level.to_string(), message));
        if self.logs.len() > 1000 {
            self.logs.remove(0);
        }
    }

    pub fn open_working_directory(&self) {
        if let Some(root) = &self.world.root_path {
            let _ = std::process::Command::new("xdg-open")
                .arg(root)
                .spawn();
        }
    }

    pub fn set_main_file(&mut self, path: std::path::PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        let path = std::fs::canonicalize(&path).unwrap_or(path);
        if !path.exists() || !path.is_file() {
            self.log("ERROR", format!("Invalid main file: {:?}", path));
            return;
        }

        if let Ok(content) = std::fs::read_to_string(&path) {
            let root = self.world.root_path.clone().unwrap_or_else(|| {
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
            });

            let vpath = typst::syntax::VirtualPath::within_root(&path, &root).unwrap_or_else(|| {
                let filename = path.file_name().unwrap_or_else(|| std::ffi::OsStr::new("main.typ"));
                typst::syntax::VirtualPath::new(filename)
            });
            let main_id = FileId::new(None, vpath);
            let source = Source::new(main_id, content);

            self.world.main_id = main_id;
            self.world.main_source = source;
            self.project.main_file_path = Some(path.clone());

            self.log("INFO", format!("Set main file: {:?}", path));

            // Also open it
            self.open_file(&path.to_string_lossy(), window, cx);
            self.compile(cx);
        } else {
            self.log("ERROR", format!("Failed to read main file: {:?}", path));
        }
    }

    pub fn change_working_directory(&mut self, new_path: std::path::PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        let new_path = std::fs::canonicalize(&new_path).unwrap_or_else(|_| new_path.clone());
        if !new_path.exists() || !new_path.is_dir() {
            self.log("ERROR", format!("Cannot change to invalid directory: {:?}", new_path));
            return;
        }

        self.world.root_path = Some(new_path.clone());
        self.project.root_path = new_path.clone();
        self.log("INFO", format!("Changed working directory to {:?}", new_path));

        if let Some(file_tree) = &self.file_tree {
            file_tree.update(cx, |tree, cx| {
                tree.set_root(new_path.clone(), cx);
            });
        }

        let main_typ = new_path.join("main.typ");
        if main_typ.exists() && main_typ.is_file() {
            self.set_main_file(main_typ, window, cx);
        } else {
            if let Ok(entries) = std::fs::read_dir(&new_path) {
                let first_typ = entries.flatten()
                    .map(|e| e.path())
                    .find(|p| p.extension().map_or(false, |ext| ext == "typ"));

                if let Some(typ_file) = first_typ {
                    self.set_main_file(typ_file, window, cx);
                } else {
                    let new_main = new_path.join("main.typ");
                    if let Err(e) = std::fs::write(&new_main, "= New Document\n") {
                        self.log("ERROR", format!("Failed to create fallback main.typ: {}", e));
                    } else {
                        self.set_main_file(new_main, window, cx);
                    }
                }
            } else {
                self.log("ERROR", format!("Failed to read directory entries for: {:?}", new_path));
            }
        }
        cx.notify();
    }

    pub fn new_document(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = "= Untitled Document\n\nStart typing here...".to_string();
        
        let mut index = 1;
        let mut path = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        loop {
            let name = format!("Untitled-{}.typ", index);
            let candidate = path.join(&name);
            if !self.editors.contains_key(&candidate) && !candidate.exists() {
                path = candidate;
                break;
            }
            index += 1;
        }

        if std::fs::write(&path, &text).is_ok() {
            self.set_main_file(path.clone(), window, cx);
            self.log("INFO", format!("Created new document: {:?}", path));
        } else {
            self.log("ERROR", format!("Failed to create new document: {:?}", path));
        }
    }

    pub fn save_file(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(path) = &self.active_editor_path {
            if let Some(editor) = self.editors.get(path).cloned() {
                let content = editor.read(cx).input.read(cx).text().to_string();
                if let Err(e) = std::fs::write(path, content) {
                    self.log("ERROR", format!("Failed to save file: {}", e));
                } else {
                    self.log("INFO", format!("Saved {:?}", path));
                    editor.update(cx, |editor, cx| {
                        editor.saved_text = editor.input.read(cx).text().clone();
                        editor.is_dirty = false;
                        cx.notify();
                    });
                }
            }
        }
    }

    pub fn save_file_as(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Typst Document", &["typ"])
            .save_file() 
        {
            if let Some(active_path) = &self.active_editor_path {
                if let Some(editor) = self.editors.get(active_path).cloned() {
                    let content = editor.read(cx).input.read(cx).text().to_string();
                    if let Err(e) = std::fs::write(&path, content) {
                        self.log("ERROR", format!("Failed to save file as: {}", e));
                    } else {
                        self.log("INFO", format!("Saved as {:?}", path));
                        editor.update(cx, |editor, cx| {
                            editor.saved_text = editor.input.read(cx).text().clone();
                            editor.is_dirty = false;
                            cx.notify();
                        });
                        self.set_main_file(path, window, cx);
                    }
                }
            }
        }
    }

    pub fn open_preferences(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let workspace_weak = cx.weak_entity();
        let pref_view = cx.new(|cx| crate::ui::components::preference_panel::PreferencePanel::new(workspace_weak, cx));
        self.dock_area.update(cx, |dock, cx| {
            dock.add_panel(
                std::sync::Arc::new(pref_view),
                gpui_component::dock::DockPlacement::Center,
                None,
                window,
                cx,
            );
        });
    }

    pub fn open_file(&mut self, path_str: &str, window: &mut Window, cx: &mut Context<Self>) {
        let path = std::path::PathBuf::from(path_str);
        if !path.exists() || !path.is_file() {
            return;
        }

        let path = std::fs::canonicalize(&path).unwrap_or(path);

        if let Some(editor) = self.editors.get(&path) {
            let editor = editor.clone();
            self.active_editor_path = Some(path.clone());
            let dock_area = self.dock_area.clone();
            window.defer(cx, move |window, cx| {
                dock_area.update(cx, |dock, cx| {
                    dock.add_panel(Arc::new(editor), gpui_component::dock::DockPlacement::Center, None, window, cx);
                });
            });
            cx.notify();
            return;
        }

        if let Ok(content) = std::fs::read_to_string(&path) {
            let workspace_weak = cx.weak_entity();
            let lsp_client = self._lsp_client.clone();
            let window_handle = self.window_handle;

            let editor_view = cx.new(|cx| {
                crate::ui::editor::SourceEditorView::new(workspace_weak, lsp_client, window_handle, window, cx, path.clone(), content)
            });

            self.editors.insert(path.clone(), editor_view.clone());
            self.active_editor_path = Some(path.clone());

            let dock_area = self.dock_area.clone();
            window.defer(cx, move |window, cx| {
                dock_area.update(cx, |dock, cx| {
                    dock.add_panel(Arc::new(editor_view), gpui_component::dock::DockPlacement::Center, None, window, cx);
                });
            });
            cx.notify();
        }
    }
    pub fn apply_editor_action_for_file(&mut self, uri: &Url, action: crate::core::editor::EditorAction, window: &mut Window, cx: &mut Context<Self>) {
        self.apply_editor_action_internal_for_file_opt(uri, action, true, true, window, cx);
    }

    pub fn apply_editor_action_from_editor(&mut self, uri: &Url, action: crate::core::editor::EditorAction, window: &mut Window, cx: &mut Context<Self>) {
        self.apply_editor_action_internal_for_file_opt(uri, action, true, false, window, cx);
    }

    pub fn apply_editor_action(&mut self, action: crate::core::editor::EditorAction, window: &mut Window, cx: &mut Context<Self>) {
        let active_uri = self.active_editor_path.as_ref().and_then(|p| {
            Url::from_str(&format!("file://{}", p.to_string_lossy())).ok()
        });
        if let Some(uri) = active_uri {
            self.apply_editor_action_internal_for_file_opt(&uri, action, true, true, window, cx);
        }
    }

    pub fn apply_virtual_editor_action(&mut self, action: crate::core::editor::EditorAction, window: &mut Window, cx: &mut Context<Self>) {
        let active_uri = self.active_editor_path.as_ref().and_then(|p| {
            Url::from_str(&format!("file://{}", p.to_string_lossy())).ok()
        });
        if let Some(uri) = active_uri {
            self.apply_editor_action_internal_for_file_opt(&uri, action, false, true, window, cx);
        }
    }

    fn apply_editor_action_internal(&mut self, action: crate::core::editor::EditorAction, record_undo: bool, window: &mut Window, cx: &mut Context<Self>) {
        let main_uri = self.project.main_file_path.as_ref().and_then(|p| {
            Url::from_str(&format!("file://{}", p.to_string_lossy())).ok()
        });
        if let Some(uri) = main_uri {
            self.apply_editor_action_internal_for_file_opt(&uri, action, record_undo, true, window, cx);
        }
    }

    fn apply_editor_action_internal_for_file_opt(
        &mut self,
        uri: &Url,
        action: crate::core::editor::EditorAction,
        record_undo: bool,
        sync_to_editor: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        use crate::core::editor::EditorAction;

        let path = uri.to_file_path().unwrap_or_else(|_| std::path::PathBuf::from(uri.path().to_string()));
        let editor_view = self.editors.get(&path).cloned();
        
        let is_active = self.active_editor_path.as_ref() == Some(&path);

        let previous_focus = window.focused(cx);
        let editor_focus = if sync_to_editor {
            editor_view.as_ref().map(|v| v.read(cx).focus_handle(cx))
        } else {
            None
        };

        match action {
            EditorAction::Edit { range, replacement, new_cursor, new_selection } => {
                let old_cursor = if is_active { self.cursor_offset } else { 0 };
                let old_selection = if is_active { self.selection.map(|s| s.start..s.end) } else { None };

                if Some(&path) == self.project.main_file_path.as_ref() {
                    let source = self.world.source_mut();
                    if record_undo {
                        let old_text = source.text()[range.clone()].to_string();
                        self.undo_manager.push(crate::core::editor::UndoEntry {
                            range: range.clone(),
                            old_text,
                            new_text: replacement.clone(),
                            old_cursor,
                            new_cursor,
                            old_selection,
                            new_selection: new_selection.clone(),
                        });
                    }
                    source.edit(range.clone(), &replacement);
                } else {
                    let root = self.world.root_path.clone().unwrap_or_else(|| PathBuf::from("."));
                    let vpath = typst::syntax::VirtualPath::within_root(&path, &root).unwrap_or_else(|| {
                        typst::syntax::VirtualPath::new(path.file_name().unwrap())
                    });
                    let id = FileId::new(None, vpath);
                    
                    let mut sources = self.world.sources.lock().unwrap();
                    if !sources.contains_key(&id) {
                        let text = std::fs::read_to_string(&path).unwrap_or_default();
                        sources.insert(id, Source::new(id, text));
                    }
                    let source = sources.get_mut(&id).unwrap();

                    if record_undo {
                        let old_text = source.text()[range.clone()].to_string();
                        self.undo_manager.push(crate::core::editor::UndoEntry {
                            range: range.clone(),
                            old_text,
                            new_text: replacement.clone(),
                            old_cursor,
                            new_cursor,
                            old_selection,
                            new_selection: new_selection.clone(),
                        });
                    }
                    source.edit(range.clone(), &replacement);
                };

                if is_active {
                    self.cursor_offset = new_cursor;
                }
                
                if sync_to_editor {
                    if let Some(view) = editor_view {
                        let range_clone = range.clone();
                        let replacement_clone = replacement.clone();
                        let selection_to_sync = new_selection.clone();
                        view.update(cx, |view, cx| {
                            view.input.update(cx, |state, cx| {
                                let text_len = state.text().len();
                                let start_byte = range_clone.start.min(text_len);
                                let end_byte = range_clone.end.min(text_len);
                                
                                let start_utf16 = state.text().offset_to_offset_utf16(start_byte);
                                let end_utf16 = state.text().offset_to_offset_utf16(end_byte);
                                
                                state.replace_text_in_range_silent(
                                    Some(start_utf16..end_utf16),
                                    &replacement_clone,
                                    window,
                                    cx
                                );
                                
                                let pos = state.text().offset_to_position(new_cursor);
                                state.set_cursor_position(pos, window, cx);

                                if let Some(sel) = selection_to_sync {
                                    state.selected_range = sel.into();
                                }
                            });
                        });
                    }
                }

                if is_active {
                    if let Some(sel) = new_selection {
                        self.selection = Some(gpui_component::input::Selection::new(sel.start, sel.end));
                    } else {
                        self.selection = None;
                    }
                }
                self.compile(cx);
            }
            EditorAction::Select { range, reversed } => {
                if is_active {
                    self.selection = Some(gpui_component::input::Selection::new(range.start, range.end));
                    self.cursor_offset = if reversed { range.start } else { range.end };
                }
                
                if sync_to_editor {
                    if let Some(view) = editor_view {
                        let range_clone = range.clone();
                        let cursor_off = if is_active { self.cursor_offset } else { if reversed { range.start } else { range.end } };
                        view.update(cx, |view, cx| {
                            view.input.update(cx, |state, cx| {
                                let pos = state.text().offset_to_position(cursor_off);
                                state.set_cursor_position(pos, window, cx);
                                state.set_selected_range(range_clone.into(), reversed, cx);
                            });
                        });
                    }
                }
                if is_active {
                    self.refresh_caret_location(cx, false);
                }
                cx.notify();
            }
            EditorAction::MoveCursor { new_cursor } => {
                if is_active {
                    self.cursor_offset = new_cursor;
                    self.selection = None;
                }
                
                if sync_to_editor {
                    if let Some(view) = editor_view {
                        view.update(cx, |view, cx| {
                            view.input.update(cx, |state, cx| {
                                if state.cursor() != new_cursor {
                                    let pos = state.text().offset_to_position(new_cursor);
                                    state.set_cursor_position(pos, window, cx);
                                }
                            });
                        });
                    }
                }

                if is_active {
                    self.update_cursor_node_info(cx);
                    self.refresh_caret_location(cx, true);
                }
                cx.notify();
            }
            EditorAction::None => {}
        }
        
        if is_active && sync_to_editor {
            if let Some(view) = self.editors.get(&path) {
                view.update(cx, |view, cx| {
                    view.input.update(cx, |state, cx| {
                        let target_sel = self.selection.unwrap_or_else(|| {
                            gpui_component::input::Selection::new(self.cursor_offset, self.cursor_offset)
                        });
                        let reversed = self.cursor_offset == target_sel.start && target_sel.start != target_sel.end;
                        if state.selected_range() != target_sel || state.selection_reversed() != reversed {
                            state.set_selected_range(target_sel, reversed, cx);
                        }
                    });
                });
            }
        }

        if let Some(prev) = previous_focus {
            if let Some(editor_focus) = editor_focus {
                if prev != editor_focus && editor_focus.is_focused(window) {
                    prev.focus(window);
                }
            }
        }
    }

    pub fn undo(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(entry) = self.undo_manager.undo() {
            let action = crate::core::editor::EditorAction::Edit {
                range: entry.range.start..(entry.range.start + entry.new_text.len()),
                replacement: entry.old_text,
                new_cursor: entry.old_cursor,
                new_selection: entry.old_selection,
            };
            self.apply_editor_action_internal(action, false, window, cx);
        }
    }

    pub fn redo(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(entry) = self.undo_manager.redo() {
            let action = crate::core::editor::EditorAction::Edit {
                range: entry.range,
                replacement: entry.new_text,
                new_cursor: entry.new_cursor,
                new_selection: entry.new_selection,
            };
            self.apply_editor_action_internal(action, false, window, cx);
        }
    }

    pub fn update_cursor_node_info(&mut self, _cx: &mut Context<Self>) {
        let source = self.world.source_ref();
        let root = LinkedNode::new(source.root());
        if let Some(node) = root.leaf_at(self.cursor_offset.min(source.text().len()), typst::syntax::Side::Before) {
            let mut info = format!("{:?}", node.kind());
            let mut current = Some(node);
            while let Some(n) = current {
                if n.kind() == SyntaxKind::Equation {
                    info = "Math Equation".to_string();
                    self.editor_context = EditorContext::Math;
                    self.context_title = "Math".to_string();
                    self.context_desc = "Mathematical structures and symbols".to_string();
                    break;
                }
                if n.kind() == SyntaxKind::CodeBlock || n.kind() == SyntaxKind::Code {
                    info = "Code Block".to_string();
                    self.editor_context = EditorContext::Code;
                    self.context_title = "Developer".to_string();
                    self.context_desc = "Rules, bindings and automation".to_string();
                    break;
                }
                current = n.parent().cloned();
                if current.is_none() {
                    self.editor_context = EditorContext::Markup;
                    self.context_title = "Home".to_string();
                    self.context_desc = "Formatting and text styles".to_string();
                }
            }
            self.selected_node_info = Some(info);
            
            let tab = match self.editor_context {
                EditorContext::Markup => crate::ui::components::ribbon::RibbonTab::Home,
                EditorContext::Math => crate::ui::components::ribbon::RibbonTab::Math,
                EditorContext::Code => crate::ui::components::ribbon::RibbonTab::Developer,
            };
            self.ribbon.update(_cx, |ribbon, cx| ribbon.set_tab(tab, cx));
        }
    }

    pub fn refresh_caret_location(&mut self, cx: &mut Context<Self>, scroll_to_caret: bool) {
        if let Some(doc) = &self.compiled_document {
            let current_source = self.world.source_ref();
            let compiled_source = self.compiled_source.as_ref().unwrap_or(current_source);
            
            let target_offset = if std::ptr::eq(current_source, compiled_source) {
                self.cursor_offset
            } else {
                map_offset_between_texts(
                    current_source.text(),
                    compiled_source.text(),
                    self.cursor_offset,
                )
            };
            
            let zoom = 1.0; 
            
            let mut best_fit: Option<(Point<Pixels>, f32, usize, usize)> = None; 
            for (idx, page) in doc.pages.iter().enumerate() {
                if let Some((pos, height, closest_dist)) = geometry::find_cursor_position(&page.frame, Point::default(), target_offset, compiled_source, zoom) {
                    if best_fit.is_none() || closest_dist < best_fit.unwrap().3 {
                        best_fit = Some((pos, height, idx, closest_dist));
                    }
                    if closest_dist == 0 {
                        break;
                    }
                }
            }

            if let Some((pos, height, page_idx, _)) = best_fit {
                self.cursor_relative_pos = Some((page_idx, pos, height));
                
                if scroll_to_caret {
                    let h = height;
                    let renderer = self._renderer.clone();
                    cx.spawn(move |_, cx: &mut AsyncApp| {
                        let cx = cx.clone();
                        async move {
                            cx.update(|cx| {
                                renderer.update(cx, |renderer, cx| {
                                    renderer.scroll_to_caret(page_idx, pos, h, cx);
                                });
                            }).ok();
                        }
                    }).detach();
                }
            }
        }
    }

    pub fn export_pdf(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        if let Some(doc) = &self.compiled_document {
            let pdf = typst_pdf::pdf(doc, &typst_pdf::PdfOptions::default()).unwrap();
            let mut path = self.world.root_path.clone().unwrap_or_else(|| PathBuf::from("."));
            path.push("output.pdf");
            if std::fs::write(&path, pdf).is_ok() {
                self.log("INFO", format!("Exported PDF to {:?}", path));
            }
        }
    }
}

impl Render for EditorWorkspace {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .cursor_default()
            .flex()
            .flex_col()
            .bg(cx.theme().background)
            .on_action(cx.listener(|this, _action: &NewDocument, window, cx| {
                this.new_document(window, cx);
            }))
            .on_action(cx.listener(|this, _action: &SaveFile, window, cx| {
                this.save_file(window, cx);
            }))
            .on_action(cx.listener(|this, _action: &SaveFileAs, window, cx| {
                this.save_file_as(window, cx);
            }))
            .on_action(cx.listener(|this, _action: &OpenPreferences, window, cx| {
                this.open_preferences(window, cx);
            }))
            .child(self.title_bar.clone())
            .child(self.ribbon.clone())
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .child(self.dock_area.clone())
            )
    }
}
