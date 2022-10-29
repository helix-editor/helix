use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::time::Duration;

use tokio;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::oneshot;
use tokio::task::{self, JoinHandle};
use tokio::time::timeout;

use helix_core::regex::Regex;

use crate::DocumentId;

// TODO get completion items len from config
const MAX_COMPLETION_ITEMS_LEN: usize = 20;
const TIMEOUT: Duration = Duration::from_millis(300);

#[allow(clippy::trivial_regex)]
static WORDS_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\w{2,}").unwrap());

pub struct Worker {
    tx: UnboundedSender<WorkerRequest>,
    handle: JoinHandle<()>,
}

impl Default for Worker {
    fn default() -> Self {
        Self::new()
    }
}

struct WorkerState {
    rx: UnboundedReceiver<WorkerRequest>,

    // TODO limit hashmap?
    // words extracted on load/save document
    doc_words: HashMap<DocumentId, HashSet<String>>,

    // words extracted on document changes assiciated to concrete lines
    // on load/save document cleared
    doc_lines_words: HashMap<DocumentId, HashMap<usize, HashSet<String>>>,
}

pub enum WorkerRequest {
    Stop,
    ExtractDocWords {
        doc_id: DocumentId,
        text: String,
    },
    ExtractDocLineWords {
        doc_id: DocumentId,
        lines: Vec<(usize, Option<String>)>,
    },
    Completion {
        prefix: String,
        tx: oneshot::Sender<Option<Vec<String>>>,
    },
}

impl std::fmt::Debug for WorkerRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkerRequest::Stop => f.write_str("Exit"),
            WorkerRequest::ExtractDocWords { doc_id, text } => f
                .debug_struct("ExtractDocWords")
                .field("doc_id", doc_id)
                .field("text_len", &text.len())
                .finish(),
            WorkerRequest::ExtractDocLineWords { doc_id, lines } => f
                .debug_struct("ExtractDocLineWords")
                .field("doc_id", doc_id)
                .field("lines_len", &lines.len())
                .finish(),
            WorkerRequest::Completion { prefix, tx: _ } => f
                .debug_struct("Completion")
                .field("prefix", prefix)
                .finish(),
        }
    }
}

impl Worker {
    pub fn new() -> Self {
        let (command_tx, command_rx) = unbounded_channel::<WorkerRequest>();

        let handle = task::spawn_blocking(|| {
            log::debug!("Worker. Start");
            let mut state = WorkerState {
                rx: command_rx,
                doc_words: HashMap::new(),
                doc_lines_words: HashMap::new(),
            };

            state.process_commands();
        });

        Worker {
            tx: command_tx,
            handle,
        }
    }

    fn send(&self, cmd: WorkerRequest) {
        if let Err(e) = self.tx.send(cmd) {
            if !self.handle.is_finished() {
                log::error!("On send command to worker: {}", e);
            }
        }
    }

    pub fn finish(&self) {
        self.send(WorkerRequest::Stop);
        self.handle.abort();
    }

    pub fn extract_words(&self, doc_id: DocumentId, text: String) {
        self.send(WorkerRequest::ExtractDocWords { doc_id, text });
    }

    pub fn extract_line_words(&self, doc_id: DocumentId, lines: Vec<(usize, Option<String>)>) {
        self.send(WorkerRequest::ExtractDocLineWords { doc_id, lines });
    }

    pub fn completion(&self, prefix: String) -> impl Future<Output = Option<Vec<String>>> {
        // TODO pool of oneshot channels
        let (tx, rx) = oneshot::channel::<Option<Vec<String>>>();

        self.send(WorkerRequest::Completion { prefix, tx });

        async move {
            match timeout(TIMEOUT, rx).await {
                Ok(r) => match r {
                    Ok(items) => items,
                    Err(e) => {
                        log::error!("On wait worker result: {}", e);
                        None
                    }
                },
                Err(e) => {
                    log::error!("On wait worker result timeout: {}", e);
                    None
                }
            }
        }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.finish();
        self.handle.abort();
    }
}

impl WorkerState {
    fn process_commands(&mut self) {
        loop {
            if let Some(command) = self.rx.blocking_recv() {
                let now = std::time::Instant::now();
                let cmd_debug = format!("{:?}", command);

                match command {
                    WorkerRequest::Stop => {
                        log::debug!("Worker Stop");
                        return;
                    }
                    WorkerRequest::ExtractDocWords { doc_id, text } => {
                        self.process_text(doc_id, text);
                    }
                    WorkerRequest::ExtractDocLineWords { doc_id, lines } => {
                        self.process_line_text(doc_id, lines);
                    }
                    WorkerRequest::Completion { prefix, tx } => {
                        self.completion(prefix, tx);
                    }
                }
                log::debug!("Worker {} took {}ms", cmd_debug, now.elapsed().as_millis());
            }
        }
    }

    fn process_text(&mut self, doc_id: DocumentId, text: String) {
        let new_words: HashSet<&str> = WORDS_REGEX.find_iter(&text).map(|w| w.as_str()).collect();

        if !new_words.is_empty() {
            let new_words: HashSet<String> = new_words.into_iter().map(String::from).collect();
            self.doc_words.insert(doc_id, new_words);
        } else if let Some(words) = self.doc_words.get_mut(&doc_id) {
            words.clear();
        }

        // reset doc lines words
        if let Some(lines) = self.doc_lines_words.get_mut(&doc_id) {
            lines.clear();
        }
    }

    fn process_line_text(&mut self, doc_id: DocumentId, lines: Vec<(usize, Option<String>)>) {
        for (line, text) in lines {
            if let Some(text) = text {
                let new_words: HashSet<&str> =
                    WORDS_REGEX.find_iter(&text).map(|w| w.as_str()).collect();

                let doc_lines = self
                    .doc_lines_words
                    .entry(doc_id)
                    .or_insert_with(HashMap::new);

                if new_words.is_empty() {
                    if let Some(doc_lines_words) = doc_lines.get_mut(&line) {
                        doc_lines_words.clear();
                    }
                } else {
                    doc_lines.insert(line, new_words.into_iter().map(String::from).collect());
                }
            } else if let Some(lines) = self.doc_lines_words.get_mut(&doc_id) {
                lines.remove(&line);
            }
        }
    }

    fn completion(&mut self, prefix: String, tx: oneshot::Sender<Option<Vec<String>>>) {
        // TODO use some index to speedup filter?

        // find in affected lines
        let items_by_lines = self
            .doc_lines_words
            .values()
            .flatten()
            .flat_map(|m| m.1)
            .filter_map(|word| {
                // skip exact already typed prefix/word and find words starts with prefix
                if word != &prefix && word.starts_with(&prefix) {
                    Some(word.as_str())
                } else {
                    None
                }
            })
            .take(MAX_COMPLETION_ITEMS_LEN)
            .collect::<HashSet<&str>>();

        let result = if items_by_lines.len() == MAX_COMPLETION_ITEMS_LEN {
            Some(
                items_by_lines
                    .into_iter()
                    .map(String::from)
                    .collect::<Vec<String>>(),
            )
        } else {
            // find in docs
            let mut items = self
                .doc_words
                .values()
                .flatten()
                .filter_map(|word| {
                    // skip exact already typed prefix/word and find words starts with prefix
                    if word != &prefix && word.starts_with(&prefix) {
                        Some(word.as_str())
                    } else {
                        None
                    }
                })
                .take(MAX_COMPLETION_ITEMS_LEN - items_by_lines.len())
                .collect::<HashSet<&str>>();

            if !items_by_lines.is_empty() {
                items.extend(items_by_lines);
            }

            if items.is_empty() {
                None
            } else {
                Some(items.into_iter().map(String::from).collect::<Vec<String>>())
            }
        };

        if tx.send(result).is_err() {
            log::error!("On send worker completion result");
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn completion() {
        use tokio::runtime::Runtime;

        // Create the runtime
        let rt = Runtime::new().unwrap();

        let doc_id = DocumentId::default();

        let _ = rt.block_on(async move {
            let worker = Worker::new();

            // buffer with text
            worker.extract_words(doc_id, "Hello".to_string());

            let items = worker.completion("H".to_string()).await;
            assert_eq!(items, Some(vec!["Hello".to_string()]));

            // add text to the same line
            worker.extract_line_words(doc_id, vec![(0, Some("Hello world".to_string()))]);

            let items = worker.completion("w".to_string()).await;
            assert_eq!(items, Some(vec!["world".to_string()]));

            // reload buffer with text
            worker.extract_words(doc_id, "Hello".to_string());

            let items = worker.completion("w".to_string()).await;
            assert_eq!(items, None);

            drop(worker);

            true
        });
    }
}
