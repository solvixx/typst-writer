use anyhow::Result;
use gpui::*;
use gpui_component::input::{InputState, CompletionProvider, HoverProvider, Rope, RopeExt};
use lsp_types::*;
use crate::core::lsp::LspClient;
use std::sync::Arc;

pub struct TypstLspProvider {
    client: Arc<LspClient>,
    file_uri: Uri,
}

impl TypstLspProvider {
    pub fn new(client: Arc<LspClient>, file_uri: Uri) -> Self {
        Self { client, file_uri }
    }
}

impl CompletionProvider for TypstLspProvider {
    fn completions(
        &self,
        _text: &Rope,
        offset: usize,
        trigger: CompletionContext,
        _window: &mut Window,
        cx: &mut Context<InputState>,
    ) -> Task<Result<CompletionResponse>> {
        let client = self.client.clone();
        let uri = self.file_uri.clone();
        
        cx.spawn(move |state: WeakEntity<InputState>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let position = state.update(&mut cx, |state, _| {
                    state.text().offset_to_position(offset)
                })?;

                let params = CompletionParams {
                    text_document_position: TextDocumentPositionParams {
                        text_document: TextDocumentIdentifier::new(uri),
                        position,
                    },
                    work_done_progress_params: Default::default(),
                    partial_result_params: Default::default(),
                    context: Some(trigger),
                };

                let resp = client.completion(params).await?;
                Ok(resp.unwrap_or(CompletionResponse::Array(vec![])))
            }
        })
    }

    fn is_completion_trigger(
        &self,
        _offset: usize,
        new_text: &str,
        _cx: &mut Context<InputState>,
    ) -> bool {
        new_text.contains('#') || new_text.contains('.')
    }
}

impl HoverProvider for TypstLspProvider {
    fn hover(
        &self,
        text: &Rope,
        offset: usize,
        _window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<Option<Hover>>> {
        let client = self.client.clone();
        let uri = self.file_uri.clone();
        let position = text.offset_to_position(offset);

        cx.spawn(move |_cx: &mut AsyncApp| async move {
            let params = HoverParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier::new(uri),
                    position,
                },
                work_done_progress_params: Default::default(),
            };

            client.hover(params).await
        })
    }
}
