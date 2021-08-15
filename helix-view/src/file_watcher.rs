use crossbeam_channel::unbounded;
use crossbeam_channel::{select, Receiver};
use notify::{recommended_watcher, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

pub struct NotifyActor {
    sender: Sender,
    watcher: (RecommendedWatcher, Receiver<NotifyEvent>),
}

impl NotifyActor {
    pub fn spawn() -> NotifyHandle {
        let (actor_tx, actor_rx) = crossbeam_channel::unbounded();
        let (tx, rx) = unbounded_channel();
        let actor = NotifyActor::new(Box::new(move |msg| tx.send(msg).unwrap()));
        let thread = jod_thread::Builder::new()
            .name("FileWatcher".to_string())
            .spawn(move || actor.run(actor_rx))
            .expect("Failed to spawn file watcher");
        NotifyHandle {
            sender: actor_tx,
            receiver: rx,
            thread,
        }
    }

    fn next_event(
        &mut self,
        receiver: &crossbeam_channel::Receiver<ActorMessage>,
    ) -> Option<ActorEvent> {
        let watcher_receiver = &self.watcher.1;
        select! {
            recv(receiver) -> it => it.ok().map(ActorEvent::Message),
            recv(watcher_receiver) -> it => Some(ActorEvent::NotifyEvent(it.unwrap())),
        }
    }

    fn new(sender: Sender) -> NotifyActor {
        let (tx, rx) = unbounded();
        let watcher: RecommendedWatcher = recommended_watcher(move |e| tx.send(e).unwrap()).unwrap();
        NotifyActor {
            sender,
            watcher: (watcher, rx),
        }
    }

    fn run(mut self, inbox: crossbeam_channel::Receiver<ActorMessage>) {
        while let Some(event) = self.next_event(&inbox) {
            match event {
                ActorEvent::Message(msg) => self.handle_message(msg),
                ActorEvent::NotifyEvent(event) => self.send(Message::NotifyEvent(event)),
            }
        }
    }

    fn handle_message(&mut self, msg: ActorMessage) {
        use ActorMessage::*;

        // probably log errors
        match msg {
            Watch(path) => self.watcher.0.watch(&path, RecursiveMode::NonRecursive),
            Unwatch(path) => self.watcher.0.unwatch(&path),
        };
    }

    fn send(&mut self, msg: Message) {
        (self.sender)(msg)
    }
}

pub enum ActorMessage {
    Watch(PathBuf),
    Unwatch(PathBuf),
}

pub enum ActorEvent {
    Message(ActorMessage),
    NotifyEvent(NotifyEvent),
}

#[derive(Debug)]
pub enum Message {
    NotifyEvent(NotifyEvent),
}

type NotifyEvent = notify::Result<notify::Event>;

type Sender = Box<dyn Fn(Message) + Send>;

#[derive(Debug)]
pub struct NotifyHandle {
    // Relative order of fields below is significant.
    pub sender: crossbeam_channel::Sender<ActorMessage>,
    thread: jod_thread::JoinHandle,
    pub receiver: UnboundedReceiver<Message>,
}
