use anyhow::Context;
use log::info;
use notify::{recommended_watcher, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::fs;
use tokio::select;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::oneshot;
use tokio::time::timeout;

fn log_notify_error(res: notify::Result<()>) {
    match res {
        Ok(()) => (),
        Err(e) => log::error!("Error from notify: {:?}", e),
    }
}

pub struct NotifyActor {
    sender: Sender,
    ignored: HashSet<PathBuf>,
    watcher: (RecommendedWatcher, UnboundedReceiver<NotifyEvent>),
}

impl NotifyActor {
    pub fn spawn(sender: Sender) -> anyhow::Result<NotifyHandle> {
        let (actor_tx, actor_rx) = unbounded_channel();
        let actor = NotifyActor::new(sender)?;
        let task = tokio::spawn(async move { actor.run(actor_rx).await });
        Ok(NotifyHandle {
            sender: actor_tx,
            // receiver: rx,
            task,
        })
    }

    async fn next_event(
        &mut self,
        receiver: &mut UnboundedReceiver<ActorMessage>,
    ) -> Option<ActorEvent> {
        let watcher_receiver = &mut self.watcher.1;
        select! {
            it = receiver.recv() => it.map(ActorEvent::Message),
            it = watcher_receiver.recv() => Some(ActorEvent::NotifyEvent(it.unwrap())),
        }
    }

    fn new(sender: Sender) -> anyhow::Result<NotifyActor> {
        let (tx, rx) = unbounded_channel();
        let watcher: RecommendedWatcher = recommended_watcher(move |e| tx.send(e).unwrap())
            .context("Failed to create file watcher")?;
        Ok(NotifyActor {
            sender,
            ignored: HashSet::new(),
            watcher: (watcher, rx),
        })
    }

    async fn run(mut self, mut inbox: UnboundedReceiver<ActorMessage>) {
        while let Some(event) = self.next_event(&mut inbox).await {
            match event {
                ActorEvent::Message(msg) => self.handle_message(msg),
                ActorEvent::NotifyEvent(event) => self.handle_notify_event(event).await,
            }
        }
    }

    fn handle_message(&mut self, msg: ActorMessage) {
        use ActorMessageKind::*;

        log::info!("Handling message: {:?}", msg);
        // probably log errors
        match msg.kind {
            Watch(path) => {
                log_notify_error(self.watcher.0.watch(&path, RecursiveMode::NonRecursive));
            }
            Unwatch(path) => {
                log_notify_error(self.watcher.0.unwatch(&path));
            }
            Ignore(path) => {
                self.ignored.insert(path);
            }
            Unignore(path) => {
                self.ignored.remove(&path);
            }
        };
        msg.done.send(()).unwrap();
    }

    async fn handle_notify_event(&mut self, msg: NotifyEvent) {
        let mut events = Events::default();
        match msg {
            Ok(it) => {
                let event = Event::from(it);
                if !self.ignored.contains(&event.path) {
                    events.insert(event);
                }
            }
            Err(e) => log::error!("Error from notify: {:?}", e),
        }

        while let Ok(Some(it)) = timeout(Duration::from_millis(200), self.watcher.1.recv()).await {
            info!("Events are: {:?}", events);
            match it {
                Ok(it) => {
                    let it = Event::from(it);
                    if self.ignored.contains(&it.path) {
                        continue;
                    }
                    events.insert(it);
                }
                Err(e) => log::error!("Error from notify: {:?}", e),
            }
        }

        for (path, op) in events.iter_mut() {
            match op {
                Operation::Delete => {
                    let meta = fs::metadata(&path).await;
                    log::info!("Meta of path {:?}: {:?}", path, meta);
                    match meta {
                        Ok(_) => {
                            log::info!("rewatching path: {:?}", path);
                            self.rewatch(path);
                            *op = Operation::Change;
                        }
                        Err(e) if e.kind() == io::ErrorKind::NotFound => (),
                        Err(e) => log::error!("Failed to stat path {:?}: {:?}", path, e),
                    }
                }
                _ => (),
            }
        }

        self.send(Message::NotifyEvents(events));
    }

    fn rewatch(&mut self, path: &Path) {
        // log_notify_error(self.watcher.0.unwatch(path));
        log_notify_error(self.watcher.0.watch(path, RecursiveMode::NonRecursive));
    }

    fn send(&mut self, msg: Message) {
        (self.sender)(msg)
    }
}

#[derive(Debug)]
pub struct ActorMessage {
    pub kind: ActorMessageKind,
    pub done: oneshot::Sender<()>,
}

#[derive(Debug)]
pub enum ActorMessageKind {
    Watch(PathBuf),
    Unwatch(PathBuf),
    Ignore(PathBuf),
    Unignore(PathBuf),
}

pub enum ActorEvent {
    Message(ActorMessage),
    NotifyEvent(NotifyEvent),
}

#[derive(Debug)]
pub enum Message {
    NotifyEvents(Events),
}

type NotifyEvent = notify::Result<notify::Event>;

#[derive(Debug, Default)]
pub struct Events(HashMap<PathBuf, Operation>);

impl Events {
    fn insert(&mut self, event: Event) {
        match self.0.get(&event.path) {
            Some(Operation::Delete | Operation::Create) => (),
            _ => {
                self.0.insert(event.path, event.op);
            }
        }
    }

    fn iter_mut(&mut self) -> impl Iterator<Item = (&PathBuf, &mut Operation)> {
        self.0.iter_mut()
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Event {
    path: PathBuf,
    op: Operation,
}

impl From<notify::Event> for Event {
    fn from(
        notify::Event {
            mut paths, kind, ..
        }: notify::Event,
    ) -> Self {
        let path = paths.drain(0..).next().unwrap();
        let op = Operation::from(kind);
        Event { path, op }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum Operation {
    Change,
    // this could be a delete or move
    Delete,
    Create,
}

impl From<notify::event::EventKind> for Operation {
    fn from(event: notify::event::EventKind) -> Self {
        use notify::event::EventKind::*;
        use notify::event::ModifyKind;

        log::info!("Converting raw event: {:?}", event);

        match event {
            Any
            | Access(_)
            | Modify(
                ModifyKind::Data(_) | ModifyKind::Any | ModifyKind::Other | ModifyKind::Metadata(_),
            )
            | Other => Operation::Change,
            Modify(ModifyKind::Name(_)) | Remove(_) => Operation::Delete,
            Create(_) => Operation::Create,
        }
    }
}

type Sender = Box<dyn Fn(Message) + Send>;

#[derive(Debug)]
pub struct NotifyHandle {
    // Relative order of fields below is significant.
    pub sender: UnboundedSender<ActorMessage>,
    #[allow(dead_code)]
    task: tokio::task::JoinHandle<()>,
}

impl NotifyHandle {
    pub async fn watch(&self, path: PathBuf) {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(ActorMessage {
                kind: ActorMessageKind::Watch(path),
                done: tx,
            })
            .unwrap();
        rx.await.unwrap();
    }

    pub async fn unwatch(&self, path: PathBuf) {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(ActorMessage {
                kind: ActorMessageKind::Unwatch(path),
                done: tx,
            })
            .unwrap();
        rx.await.unwrap();
    }

    pub async fn ignore(&self, path: PathBuf) {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(ActorMessage {
                kind: ActorMessageKind::Ignore(path),
                done: tx,
            })
            .unwrap();
        rx.await.unwrap();
    }

    pub async fn unignore(&self, path: PathBuf) {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(ActorMessage {
                kind: ActorMessageKind::Unignore(path),
                done: tx,
            })
            .unwrap();
        rx.await.unwrap();
    }
}