use file_lock::{FileLock, FileOptions};

use super::{event::Event, event::EventBuilder, EventId, User, UserId};
use std::{
    collections::BTreeMap,
    fs::{create_dir_all, File},
    io::{self, BufReader, BufWriter, Read},
    path::{Path, PathBuf},
};

/// Main interface to interact with the internal files
pub struct Database {
    base_path: PathBuf,
}
impl Database {
    /// Creates a new database representation, if using a filesystem schema,
    /// it accepts the relative path where the file and folders will be created,
    /// does not create a new folder to contain the rest of the database.
    ///
    /// ### Usage
    /// ```no_run
    /// # use nari::models::Database;
    /// # fn main() -> std::io::Result<()> {
    /// // Creates the database files and saves its contents in "db" //
    /// let db = Database::new("./db")?;
    /// # Ok(())
    /// # }
    /// ```
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
    /// Creates and adds an user to the database, returns the created user.
    ///
    /// ## Usage
    /// ```no_run
    /// # use nari::models::Database;
    /// # use nari::models::UserId;
    /// # fn main() {
    /// # let db = Database::new("./db/").unwrap();
    /// let alice = db.create_user(UserId(42), "Alice");
    /// # }
    /// ```
    pub fn create_user(&self, id: UserId, name: &str) -> User {
        let user = User::new(id, name);
        self.add_user(user.clone());
        user
    }
    /// Adds an already created user to the database, consumes the user.
    ///
    /// ## Usage
    /// ```no_run
    /// # use nari::models::Database;
    /// # use nari::models::UserId;
    /// # use nari::models::User;
    /// # fn main() {
    /// # let db = Database::new("./db/").unwrap();
    /// let alice = User::new(UserId(42), "Alice");
    /// db.add_user(alice);
    /// # }
    /// ```
    pub fn add_user(&self, user: User) {
        let buf = self.open_buf_writer(user.id.0, "users").unwrap();
        ron::ser::to_writer(buf, &user).unwrap();
    }
    /// Search for a user in the database, returns the user if found.
    ///
    /// ## Usage
    /// ```no_run
    /// # use nari::models::Database;
    /// use nari::models::UserId;
    /// # fn main() {
    /// # let db = Database::new("./db/").unwrap();
    /// let alice = db.create_user(UserId(42), "Alice");
    /// assert_eq!(alice, db.fetch_user(UserId(42)));
    /// # }
    /// ```
    pub fn fetch_user(&self, id: UserId) -> User {
        let buf = self.open_buf_reader(id.0, "users").unwrap();
        ron::de::from_reader(buf).unwrap()
    }

    /// Returns an [`EventBuilder`], with the minimum information required.
    ///
    /// Takes an [`EventId`] which must represent an unique u64 value, the name for the event
    /// and a u64 number representing an unix timestamp of when should it fire.
    ///  
    /// It doesn't add the event to the database until it is built and manually
    /// added.
    ///
    /// ## Usage
    /// ```no_run
    /// # use nari::models::Database;
    /// # use nari::models::EventId;
    /// # fn main() {
    /// # let db = Database::new("./db/").unwrap();
    /// // We build a new event for Alice's birthday and manually add it to the database
    /// let birthday = db.build_event(EventId(14), "Alice's Birthday", 123456789)
    ///         .build()
    ///         .save_to_db(&db);
    /// # }
    /// ```
    pub fn build_event(&self, id: EventId, name: &str, next_occurence: u64) -> EventBuilder {
        EventBuilder::new(id, name, next_occurence)
    }
    /// Adds an already created event to the database, consumes the event.
    ///
    /// It adds it to the database automatically.
    ///
    /// ## Usage
    /// ```no_run
    /// # use nari::models::Database;
    /// # use nari::models::UserId;
    /// # use nari::models::EventId;
    /// # use nari::models::User;
    /// # use nari::models::event::EventBuilder;
    /// # fn main() {
    /// # let db = Database::new("./db/").unwrap();
    /// let alice = User::new(UserId(42), "Alice");
    /// let bob = User::new(UserId(43), "Bob");
    /// let alices_birthday = EventBuilder::new(EventId(42), "Alice's Birthday", 123456789)
    ///         .description("Today is Alice's birthday! 🎉")
    ///         .users([alice.id, bob.id])
    ///         .build();
    /// db.add_event(alices_birthday);
    /// # }
    /// ```
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
    /// Search for a event in the database, returns the event if found.
    ///
    /// ## Usage
    /// ```no_run
    /// # use nari::models::Database;
    /// # use nari::models::EventId;
    /// # use nari::models::event::EventBuilder;
    /// # fn main() {
    /// # let db = Database::new("./db/").unwrap();
    /// let alices_birthday = EventBuilder::new(EventId(10), "Alice's Birthday", 123456789)
    ///         .build();
    /// db.add_event(alices_birthday.clone());
    /// assert_eq!(alices_birthday, db.fetch_event(EventId(10)));
    /// # }
    /// ```
    pub fn fetch_event(&self, id: EventId) -> Event {
        let buf = self.open_buf_reader(id.0, "events").unwrap();
        ron::de::from_reader(buf).unwrap()
    }

    /// Takes an event and adds it to any amount of users, it can take any
    /// collection of [`User`] as long as it implements the [`IntoIterator`] trait.
    ///
    /// It saves the event and the users to the database automatically.
    ///
    /// [`User`] tracks what events it is in with a [`HashSet`](std::collections::HashSet) internally. This means
    /// that if any two or more events have the same [`EventId`], they won't repeat
    /// and only the latest one created will be used.
    ///   
    /// ## Usage
    /// ```no_run
    /// # use nari::models::Database;
    /// # use nari::models::event::Event;
    /// # use nari::models::{EventId, UserId};
    /// # use nari::models::User;
    /// # use nari::models::event::EventBuilder;
    /// # fn main() {
    /// # let db = Database::new("./db/").unwrap();
    /// # let alice = User::new(UserId(42), "Alice");
    /// # let bob = User::new(UserId(43), "Bob");
    /// let alices_birthday = EventBuilder::new(EventId(42), "Alice's Birthday", 123456789)
    ///         .description("Today is Alice's birthday! 🎉")
    ///         .build();
    /// db.add_event_to_users(alices_birthday, [alice, bob]);
    /// # }
    /// ```
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
    /// Takes an user and adds it to any amount of events, it can take any
    /// collection of [`Event`] as long as it implements the [`IntoIterator`] trait.
    ///
    /// It saves the user and the events to the database automatically.
    ///
    /// [`Event`] tracks what users it has with a [`HashSet`](std::collections::HashSet) internally. This means
    /// that if any two or more users have the same [`UserId`], they won't repeat
    /// and only the lastest one created will be used.
    ///
    /// ## Usage
    /// ```no_run
    /// # use nari::models::Database;
    /// # use nari::models::event::Event;
    /// # use nari::models::UserId;
    /// # use nari::models::event::EventBuilder;
    /// # use nari::models::EventId;
    /// # use nari::models::User;
    /// # fn main() {
    /// # let db = Database::new("./db/").unwrap();
    /// let bob = User::new(UserId(43), "Bob");
    /// let alices_birthday = EventBuilder::new(EventId(42), "Alice's Birthday", 123456789)
    ///        .build();
    /// let job_meeting = EventBuilder::new(EventId(43), "A job meeting", 123456789)
    ///       .build();  
    /// let park_hangout = EventBuilder::new(EventId(44), "Park hangout", 123456789)
    ///       .build();   
    /// db.add_user_to_events(bob, [alices_birthday, job_meeting, park_hangout])
    /// # }
    /// ```
    pub fn add_user_to_events<I>(&self, mut user: User, events: I)
    where
        I: IntoIterator<Item = Event>,
    {
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
    /// Reads the whole database and replaces the current event queue of future events
    /// with the one read. It should fix any possible desync problems that may have arisen.
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
