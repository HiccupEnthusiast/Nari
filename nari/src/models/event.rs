use std::{
    collections::{BTreeMap, HashSet},
    fs::File,
    io::{BufReader, Read},
    path::Path,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use file_lock::{FileLock, FileOptions};
use notify::{event::ModifyKind::Data, EventKind, RecommendedWatcher, Watcher};
use serde::{Deserialize, Serialize};
use tokio::{
    sync::mpsc,
    time::{interval, Duration},
};

use super::{Database, EventId, UserId};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Event {
    pub id: EventId,
    pub name: String,
    pub description: String,
    pub next_occurence: u64,
    pub users: HashSet<UserId>,
    pub repeats: Repeatability,
    pub priority: Priority,
}
impl Event {
    pub fn save_to_db(self, db: &Database) {
        db.add_event(self);
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub enum Repeatability {
    Yearly,
    Biyearly,
    Quarterly,
    Monthly,
    Bimonthly,
    Weekly,
    Daily,
    Hourly,
    #[default]
    Never,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub enum Priority {
    Urgent,
    VeryHigh,
    High,
    Medium,
    #[default]
    Low,
    Minimal,
}

#[derive(Debug, Default)]
pub struct EventBuilder {
    id: EventId,
    name: String,
    description: String,
    next_occurence: u64,
    users: HashSet<UserId>,
    repeats: Repeatability,
    priority: Priority,
}
impl EventBuilder {
    pub fn new(id: EventId, name: &str, next_occurence: u64) -> Self {
        Self {
            id,
            name: String::from(name),
            next_occurence,
            ..Self::default()
        }
    }
    pub fn description(mut self, desc: &str) -> Self {
        self.description = String::from(desc);
        self
    }
    pub fn users(mut self, users: HashSet<UserId>) -> Self {
        self.users = users;
        self
    }
    pub fn repeats(mut self, repeats: Repeatability) -> Self {
        self.repeats = repeats;
        self
    }
    pub fn priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }
    pub fn build(self) -> Event {
        Event {
            id: self.id,
            name: self.name,
            description: self.description,
            next_occurence: self.next_occurence,
            users: self.users,
            repeats: self.repeats,
            priority: self.priority,
        }
    }
}
#[non_exhaustive]
#[derive(Debug)]
pub struct EventListener {
    sender: mpsc::Sender<Event>,
    refresh_rate: u64,
}
impl EventListener {
    pub fn new(sender: mpsc::Sender<Event>, refresh_rate: u64) -> Self {
        Self {
            sender,
            refresh_rate,
        }
    }
    pub async fn start(self) {
        // this may look dirty, cuz it is, please send help, i am not fit for this
        let options = FileOptions::new().read(true).write(true).create(true);
        let mut filelock = FileLock::lock("./db/event_cache.ron", true, options).unwrap();
        let mut bytes = vec![];
        filelock.file.read_to_end(&mut bytes).unwrap();
        let event_cache: BTreeMap<u64, u64> = ron::de::from_bytes(&bytes).unwrap();
        let event_cache = Arc::new(Mutex::new(event_cache));
        let copy = Arc::clone(&event_cache);

        let _watcher = tokio::spawn(async move {
            let event_cache = Arc::clone(&event_cache);

            let (tx, rx) = std::sync::mpsc::channel();
            let mut w = RecommendedWatcher::new(tx, notify::Config::default()).unwrap();
            w.watch(
                Path::new("./db/event_cache.ron"),
                notify::RecursiveMode::Recursive,
            )
            .unwrap();
            while let Ok(f_ev) = rx.recv() {
                if let Ok(file_event) = f_ev {
                    match file_event.kind {
                        EventKind::Modify(Data(_)) => {
                            let options = FileOptions::new().read(true).write(true).create(true);
                            let mut filelock =
                                FileLock::lock("./db/event_cache.ron", true, options).unwrap();
                            let mut bytes = vec![];
                            filelock.file.read_to_end(&mut bytes).unwrap();
                            let mut event_cache = event_cache.lock().unwrap();
                            *event_cache =
                                ron::de::from_bytes::<BTreeMap<u64, u64>>(&bytes).unwrap();
                        }
                        _ => (),
                    }
                }
            }
        });
        let mut interval = interval(Duration::from_millis(self.refresh_rate));
        let mut ids: Vec<u64> = vec![];
        loop {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            if Self::has_passed_event(now, &copy.lock().unwrap()) {
                let mut lock = copy.lock().unwrap();
                for (_, id) in lock.range(..now) {
                    ids.push(*id);
                }
                lock.retain(|k, _| *k >= now);
                let writer = File::create("./db/event_cache.ron").unwrap();
                ron::ser::to_writer(writer, &*lock).unwrap();
                drop(lock);
            }
            if !ids.is_empty() {
                for id in &ids {
                    let buf = BufReader::new(File::open(format!("./db/events/{id}.ron")).unwrap());
                    let e: Event = ron::de::from_reader(buf).unwrap();
                    self.sender.send(e.clone()).await.unwrap();
                }
                ids.clear();
            }
            interval.tick().await;
        }
    }
    fn has_passed_event(now: u64, events: &BTreeMap<u64, u64>) -> bool {
        if let Some((k, _)) = events.first_key_value() {
            *k <= now
        } else {
            false
        }
    }
}
