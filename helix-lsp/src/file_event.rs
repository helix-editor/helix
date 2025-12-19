use std::collections::hash_map;
use std::mem::take;
use std::path::{is_separator, Path};
use std::{collections::HashMap, path::PathBuf, sync::Weak};

use globset::{Glob, GlobSet};
use helix_core::file_watcher::{EventType, Events, FileSystemDidChange};
use helix_event::register_hook;
use helix_lsp_types::WatchKind;
use tokio::sync::mpsc;

use crate::{lsp, Client, LanguageServerId};

enum Event {
    /// file written by helix, special cased to not wait on FS
    FileWritten {
        path: PathBuf,
    },
    FileWatcher(Events),
    Register {
        client: Weak<Client>,
        registration_id: String,
        options: lsp::DidChangeWatchedFilesRegistrationOptions,
    },
    Unregister {
        client_id: LanguageServerId,
        registration_id: String,
    },
    RemoveClient {
        client_id: LanguageServerId,
    },
}

struct ClientState {
    client: Weak<Client>,
    registerations: HashMap<String, u32>,
    pending: Vec<lsp::FileEvent>,
}

#[derive(Debug, Clone)]
struct Interest {
    glob: Glob,
    server: LanguageServerId,
    id: u32,
    flags: WatchKind,
}

struct State {
    clients: HashMap<LanguageServerId, ClientState>,
    glob_matcher: GlobSet,
    interest: Vec<Interest>,
    // used for matching to avoid reallocation
    candidates: Vec<usize>,
}
impl State {
    fn notify<'a>(&mut self, events: impl Iterator<Item = (&'a Path, EventType)> + Clone) {
        for (path, ty) in events {
            let (interest_kind, notification_type) = match ty {
                EventType::Create => (WatchKind::Create, lsp::FileChangeType::CREATED),
                EventType::Delete => (WatchKind::Delete, lsp::FileChangeType::DELETED),
                EventType::Modified => (WatchKind::Change, lsp::FileChangeType::CHANGED),
                EventType::Tempfile => continue,
            };
            self.glob_matcher.matches_into(path, &mut self.candidates);
            for interest in self.candidates.drain(..) {
                let interest = &self.interest[interest];
                if !interest.flags.contains(interest_kind) {
                    continue;
                }
                let Ok(uri) = lsp::Url::from_file_path(path) else {
                    continue;
                };
                let event = lsp::FileEvent {
                    uri,
                    typ: notification_type,
                };
                self.clients
                    .get_mut(&interest.server)
                    .unwrap()
                    .pending
                    .push(event);
            }
        }
        for client_state in self.clients.values_mut() {
            if client_state.pending.is_empty() {
                continue;
            }
            let Some(client) = client_state.client.upgrade() else {
                continue;
            };
            log::debug!(
                "Sending didChangeWatchedFiles notification to client '{}'",
                client.name()
            );
            client.did_change_watched_files(take(&mut client_state.pending));
        }
    }

    fn purge_client(&mut self, id: LanguageServerId) {
        self.clients.remove(&id);
        let interest = self
            .interest
            .iter()
            .filter(|it| it.server != id)
            .cloned()
            .collect();
        self.rebuild_globmatcher(interest);
    }

    fn rebuild_globmatcher(&mut self, interest: Vec<Interest>) {
        let mut builder = GlobSet::builder();
        for interest in &interest {
            builder.add(interest.glob.clone());
        }
        match builder.build() {
            Ok(glob_matcher) => {
                self.glob_matcher = glob_matcher;
                self.interest = interest;
            }
            Err(err) => {
                log::error!("failed to build glob matcher for file watching: ({err})",);
            }
        }
    }
}

impl Default for State {
    fn default() -> State {
        State {
            clients: Default::default(),
            glob_matcher: Default::default(),
            interest: Default::default(),
            candidates: Vec::with_capacity(32),
        }
    }
}

/// The Handler uses a dedicated tokio task to respond to file change events by
/// forwarding changes to LSPs that have registered for notifications with a
/// matching glob.
///
/// When an LSP registers for the DidChangeWatchedFiles notification, the
/// Handler is notified by sending the registration details in addition to a
/// weak reference to the LSP client. This is done so that the Handler can have
/// access to the client without preventing the client from being dropped if it
/// is closed and the Handler isn't properly notified.
#[derive(Clone, Debug)]
pub struct Handler {
    tx: mpsc::UnboundedSender<Event>,
}

impl Default for Handler {
    fn default() -> Self {
        Self::new()
    }
}

impl Handler {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(Self::run(rx));
        let tx_ = tx.clone();
        register_hook!(move |event: &mut FileSystemDidChange| {
            let _ = tx_.send(Event::FileWatcher(event.fs_events.clone()));
            Ok(())
        });
        Self { tx }
    }

    pub fn register(
        &self,
        client: Weak<Client>,
        registration_id: String,
        options: lsp::DidChangeWatchedFilesRegistrationOptions,
    ) {
        let _ = self.tx.send(Event::Register {
            client,
            registration_id,
            options,
        });
    }

    pub fn unregister(&self, client_id: LanguageServerId, registration_id: String) {
        let _ = self.tx.send(Event::Unregister {
            client_id,
            registration_id,
        });
    }

    pub fn file_changed(&self, path: PathBuf) {
        let _ = self.tx.send(Event::FileWritten { path });
    }

    pub fn remove_client(&self, client_id: LanguageServerId) {
        let _ = self.tx.send(Event::RemoveClient { client_id });
    }

    async fn run(mut rx: mpsc::UnboundedReceiver<Event>) {
        let mut state = State::default();
        while let Some(event) = rx.recv().await {
            match event {
                Event::FileWatcher(events) => {
                    let events = events
                        .iter()
                        .map(|event| (event.path.as_std_path(), event.ty));
                    state.notify(events);
                }
                Event::FileWritten { path } => {
                    log::debug!("Received file event for {:?}", &path);
                    state.notify([(&*path, EventType::Modified)].iter().cloned());
                }
                Event::Register {
                    client,
                    registration_id,
                    options: ops,
                } => {
                    let Some(client_) = client.upgrade() else {
                        continue;
                    };
                    log::debug!(
                        "Registering didChangeWatchedFiles for client '{}' with id '{}'",
                        client_.name(),
                        registration_id
                    );

                    if !state
                        .clients
                        .get(&client_.id())
                        .is_some_and(|state| !state.client.ptr_eq(&client))
                    {
                        state.purge_client(client_.id());
                    }
                    let entry = state
                        .clients
                        .entry(client_.id())
                        .or_insert_with(|| ClientState {
                            client: client.clone(),
                            registerations: HashMap::with_capacity(8),
                            pending: Vec::with_capacity(32),
                        });
                    entry.client = client;
                    let next_id = u32::try_from(entry.registerations.len()).unwrap();
                    let (mut interest, id) = match entry.registerations.entry(registration_id) {
                        hash_map::Entry::Occupied(entry) => {
                            let id = *entry.get();
                            let mut interest = Vec::with_capacity(state.interest.len());
                            interest.extend(
                                state
                                    .interest
                                    .iter()
                                    .filter(|it| it.server != client_.id() || it.id != id)
                                    .cloned(),
                            );
                            (interest, id)
                        }
                        hash_map::Entry::Vacant(entry) => {
                            entry.insert(next_id);
                            (state.interest.clone(), next_id)
                        }
                    };
                    for watcher in ops.watchers {
                        if watcher.kind.is_some_and(|flags| flags.is_empty()) {
                            continue;
                        }
                        let glob = match watcher.glob_pattern {
                            helix_lsp_types::GlobPattern::String(pattern) => pattern,
                            helix_lsp_types::GlobPattern::Relative(relative_pattern) => {
                                let base_url = match relative_pattern.base_uri {
                                    helix_lsp_types::OneOf::Left(folder) => folder.uri,
                                    helix_lsp_types::OneOf::Right(url) => url,
                                };
                                let Ok(mut base_dir) = base_url.to_file_path() else {
                                    log::error!(
                                        "{} provided invalid URL for watching '{base_url}'",
                                        client_.name(),
                                    );
                                    continue;
                                };
                                if let Ok(dir) = base_dir.canonicalize() {
                                    base_dir = dir
                                }
                                let Ok(mut base_dir) = base_dir.into_os_string().into_string()
                                else {
                                    log::error!(
                                        "{} provided invalid URL for watching '{base_url}' (must be valid utf-8)",
                                        client_.name(),
                                    );
                                    continue;
                                };
                                if !base_dir.chars().next_back().is_some_and(is_separator) {
                                    base_dir.push('/');
                                }
                                base_dir.push_str(&relative_pattern.pattern);
                                base_dir
                            }
                        };
                        match Glob::new(&glob) {
                            Ok(glob) => {
                                interest.push(Interest {
                                    glob,
                                    server: client_.id(),
                                    id,
                                    flags: watcher.kind.unwrap_or(WatchKind::all()),
                                });
                            }
                            Err(err) => {
                                log::error!(
                                    "{} provided invalid glob for watching '{glob}': ({err})",
                                    client_.name(),
                                );
                            }
                        }
                    }
                    state.rebuild_globmatcher(interest);
                }
                Event::Unregister {
                    client_id,
                    registration_id,
                } => {
                    let Some(client_state) = state.clients.get_mut(&client_id) else {
                        return;
                    };
                    let Some(id) = client_state.registerations.remove(&*registration_id) else {
                        return;
                    };
                    let interest = state
                        .interest
                        .iter()
                        .filter(|it| it.server != client_id || it.id != id)
                        .cloned()
                        .collect();
                    state.rebuild_globmatcher(interest);
                }
                Event::RemoveClient { client_id } => {
                    log::debug!("Removing LSP client: {client_id}");
                    state.purge_client(client_id);
                }
            }
        }
    }
}
