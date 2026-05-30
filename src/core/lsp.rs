use anyhow::{anyhow, Result};
use futures::channel::oneshot;
use gpui::{App, Task};
use lsp_types::{
    notification::{self, Notification},
    request::{self, Request},
    *,
};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Write},
    process::{ChildStdin, Command, Stdio},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};

pub struct LspClient {
    stdin: Mutex<ChildStdin>,
    request_index: AtomicU64,
    pending_requests: Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value>>>>>,
    _read_task: Task<()>,
}

impl LspClient {
    pub fn new(binary_path: &str, args: &[String], cx: &mut App) -> Result<Arc<Self>> {
        let mut child = Command::new(binary_path)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdin = child.stdin.take().ok_or_else(|| anyhow!("failed to take stdin"))?;
        let stdout = child.stdout.take().ok_or_else(|| anyhow!("failed to take stdout"))?;
        let stderr = child.stderr.take().ok_or_else(|| anyhow!("failed to take stderr"))?;

        let pending_requests = Arc::new(Mutex::new(HashMap::<u64, oneshot::Sender<Result<Value>>>::new()));
        let pending_requests_clone = pending_requests.clone();

        let read_task = cx.background_executor().spawn(async move {
            let mut reader = BufReader::new(stdout);
            loop {
                match read_message(&mut reader) {
                    Ok(Some(message)) => {
                        handle_message(message, &pending_requests_clone);
                    }
                    Ok(None) => break,
                    Err(_e) => {
                        break;
                    }
                }
            }
        });

        // Suppress verbose stderr logs from LSP by consuming the stream
        cx.background_executor().spawn(async move {
            let reader = BufReader::new(stderr);
            for _ in reader.lines() {
                // Discard logs to avoid cluttering stderr
            }
        }).detach();

        Ok(Arc::new(Self {
            stdin: Mutex::new(stdin),
            request_index: AtomicU64::new(0),
            pending_requests,
            _read_task: read_task,
        }))
    }

    pub async fn request<R: Request>(&self, params: R::Params) -> Result<R::Result> {
        let id = self.request_index.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = oneshot::channel();

        {
            let mut pending = self.pending_requests.lock().unwrap();
            pending.insert(id, tx);
        }

        let message = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": R::METHOD,
            "params": params,
        });

        self.send_message(&message)?;

        let response = rx.await.map_err(|_| anyhow!("LSP response channel closed"))??;
        let result = serde_json::from_value(response)?;
        Ok(result)
    }

    pub fn notify<N: Notification>(&self, params: N::Params) -> Result<()> {
        let message = json!({
            "jsonrpc": "2.0",
            "method": N::METHOD,
            "params": params,
        });

        self.send_message(&message)
    }

    fn send_message(&self, message: &Value) -> Result<()> {
        let content = serde_json::to_string(message)?;
        let mut stdin = self.stdin.lock().unwrap();
        write!(stdin, "Content-Length: {}\r\n\r\n{}", content.len(), content)?;
        stdin.flush()?;
        Ok(())
    }

    pub async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        self.request::<request::Initialize>(params).await
    }

    pub fn initialized(&self, params: InitializedParams) -> Result<()> {
        self.notify::<notification::Initialized>(params)
    }

    pub fn did_open(&self, params: DidOpenTextDocumentParams) -> Result<()> {
        self.notify::<notification::DidOpenTextDocument>(params)
    }

    pub fn did_change(&self, params: DidChangeTextDocumentParams) -> Result<()> {
        self.notify::<notification::DidChangeTextDocument>(params)
    }

    pub fn did_close(&self, params: DidCloseTextDocumentParams) -> Result<()> {
        self.notify::<notification::DidCloseTextDocument>(params)
    }

    pub async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        self.request::<request::Completion>(params).await
    }

    pub async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        self.request::<request::HoverRequest>(params).await
    }

    pub async fn shutdown(&self) -> Result<()> {
        self.request::<request::Shutdown>(()).await
    }

    pub fn exit(&self) -> Result<()> {
        self.notify::<notification::Exit>(())
    }
}

fn read_message<R: BufRead>(reader: &mut R) -> Result<Option<Value>> {
    let mut line = String::new();
    let mut content_length = 0;

    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        if line == "\r\n" {
            break;
        }
        if let Some(len) = line.strip_prefix("Content-Length: ") {
            content_length = len.trim().parse::<usize>()?;
        }
    }

    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body)?;
    let message = serde_json::from_slice(&body)?;
    Ok(Some(message))
}

type PendingRequests = Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value>>>>>;

fn handle_message(message: Value, pending_requests: &PendingRequests) {
    if let Some(id) = message.get("id") {
        let id_u64 = match id {
            Value::Number(n) => n.as_u64(),
            Value::String(s) => s.parse::<u64>().ok(),
            _ => None,
        };

        if let Some(id) = id_u64 {
            let mut pending = pending_requests.lock().unwrap();
            if let Some(tx) = pending.remove(&id) {
                if let Some(error) = message.get("error") {
                    let _ = tx.send(Err(anyhow!("LSP error: {}", error)));
                } else if let Some(result) = message.get("result") {
                    let _ = tx.send(Ok(result.clone()));
                } else {
                    let _ = tx.send(Ok(Value::Null));
                }
            }
        }
    }
}
