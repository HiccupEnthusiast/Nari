use file_lock::{FileLock, FileOptions};

use super::{event::Event, event::EventBuilder, EventId, User, UserId};
use std::{
    collections::{BTreeMap, HashSet},
    fs::{create_dir_all, File},
    io::{self, BufReader, BufWriter, Read},
    path::{Path, PathBuf},
};

pub struct Database {
    base_path: PathBuf,
}
impl Database {
    pub fn new<P>(base_path: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        let base_path = base_path.as_ref().to_path_buf();
        create_dir_all(base_path.join("users"))?;
        create_dir_all(base_path.join("events"))?;
        if let Ok(f) = File::options()
            .write(true)
            .create_new(true)
            .open(base_path.join("event_cache.ron"))
        {
            let buf = BufWriter::new(f);
            let mut tree: BTreeMap<u64, u64> = BTreeMap::new();
            tree.insert(u64::MAX, 0);
            ron::ser::to_writer(buf, &tree).unwrap();
        }

        Ok(Self { base_path })
    }
    pub fn create_user(&self, id: UserId, name: &str) -> User {
        let user = User::new(id, name);
        self.add_user(user.clone());
        user
    }
    pub fn add_user(&self, user: User) {
        let buf = self.open_buf_writer(user.id.0, "users").unwrap();
        ron::ser::to_writer(buf, &user).unwrap();
    }
    pub fn fetch_user(&self, id: UserId) -> User {
        let buf = self.open_buf_reader(id.0, "users").unwrap();
        ron::de::from_reader(buf).unwrap()
    }

    pub fn build_event(&self, id: EventId, name: &str, next_occurence: u64) -> EventBuilder {
        EventBuilder::new(id, name, next_occurence)
    }
    pub fn add_event(&self, event: Event) {
        self.add_event_to_cache(&event);
        let buf = self.open_buf_writer(event.id.0, "events").unwrap();
        ron::ser::to_writer(buf, &event).unwrap();
        if !event.users.is_empty() {
            let mut users = vec![];
            for u in event.users.iter() {
                let user = self.fetch_user(*u);
                users.push(user);
            }
            self.add_event_to_users(event, users)
        }
    }
    pub fn fetch_event(&self, id: EventId) -> Event {
        let buf = self.open_buf_reader(id.0, "events").unwrap();
        ron::de::from_reader(buf).unwrap()
    }

    pub fn add_event_to_users<I>(&self, mut event: Event, users: I)
    where
        I: IntoIterator<Item = User>,
    {
        self.add_event_to_cache(&event);

        for mut u in users {
            u.events.insert(event.id);
            let buf = self.open_buf_writer(u.id.0, "users").unwrap();
            ron::ser::to_writer(buf, &u).unwrap();

            event.users.insert(u.id);
        }
        let buf = self.open_buf_writer(event.id.0, "events").unwrap();
        ron::ser::to_writer(buf, &event).unwrap()
    }
    pub fn add_user_to_events(&self, mut user: User, events: HashSet<Event>) {
        for mut e in events {
            self.add_event_to_cache(&e);
            e.users.insert(user.id);
            let buf = self.open_buf_writer(e.id.0, "events").unwrap();
            ron::ser::to_writer(buf, &e).unwrap();

            user.events.insert(e.id);
        }
        let buf = self.open_buf_writer(user.id.0, "users").unwrap();
        ron::ser::to_writer(buf, &user).unwrap();
    }
    pub fn rewrite_cache(&self) {
        let dir = std::fs::read_dir(self.base_path.join("events")).unwrap();
        let mut events = BTreeMap::new();
        for entry in dir {
            let path = entry.unwrap().path();
            if path.is_file() {
                let file = File::open(path).unwrap();
                let buf = BufReader::new(file);
                let ev: Event = ron::de::from_reader(buf).unwrap();
                events.insert(ev.next_occurence, ev.id.0);
            }
        }
        let buf = BufWriter::new(File::open(self.base_path.join("event_cache.bin")).unwrap());
        ron::ser::to_writer(buf, &events).unwrap();
    }

    fn open_buf_reader(&self, id: u64, folder: &str) -> io::Result<BufReader<File>> {
        let path: PathBuf = [
            &self.base_path,
            &PathBuf::from(folder),
            &PathBuf::from(format!("{id}.ron")),
        ]
        .iter()
        .collect();

        Ok(BufReader::new(File::open(path)?))
    }
    fn open_buf_writer(&self, id: u64, folder: &str) -> io::Result<BufWriter<File>> {
        let path: PathBuf = [
            &self.base_path,
            &PathBuf::from(folder),
            &PathBuf::from(format!("{id}.ron")),
        ]
        .iter()
        .collect();

        Ok(BufWriter::new(File::create(path)?))
    }
    fn add_event_to_cache(&self, ev: &Event) {
        let options = FileOptions::new().read(true).write(true).create(true);
        let mut filelock =
            FileLock::lock(self.base_path.join("event_cache.ron"), true, options).unwrap();

        let mut bytes = vec![];
        filelock.file.read_to_end(&mut bytes).unwrap();
        let mut tree: BTreeMap<u64, u64> = ron::de::from_bytes(&bytes).unwrap();
        tree.insert(ev.next_occurence, ev.id.0);

        let options = FileOptions::new().truncate(true).write(true).create(true);
        let filelock =
            FileLock::lock(self.base_path.join("event_cache.ron"), true, options).unwrap();

        let writer = BufWriter::new(&filelock.file);
        ron::ser::to_writer(writer, &tree).unwrap()
    }
}
