use std::time::{Duration, SystemTime};

use nari::models::{
    event::{Event, EventBuilder, EventListener},
    Database, EventId, User, UserId,
};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    // Create a new database, with it we also create recursively the necessary folders.
    let db = Database::new("db/").unwrap();
    // We can initialize users manually
    let alice = User::new(UserId(1), "Alice");
    db.add_user(alice.clone());
    // Or create them from the database
    let bob = db.create_user(UserId(2), "Bob");

    // Same with events, using the builders is recommended
    // We provide an u64 unix timestamp
    let in_two_seconds = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .checked_add(Duration::from_secs(2))
        .unwrap()
        .as_secs();
    db.build_event(EventId(1), "Alice's Birthday", in_two_seconds)
        .users([alice.id])
        .build()
        .save_to_db(&db);

    let ev = EventBuilder::new(EventId(2), "Monthly club meeting", in_two_seconds + 2)
        .description("Montly updates of the activites related to the club")
        .build();
    db.add_event_to_users(ev, [alice, bob]);

    // We use channels to listen to any incomming event
    let (event_transmiter, mut event_listener) = mpsc::channel(16);
    let listener = EventListener::new(event_transmiter, 500);
    tokio::spawn(listener.start());

    // Since an `Event` can be represented in multiple equally valid and meaningful ways, it does not implement a Display
    // We need to use the newtype pattern or delegate the formatting to a function
    while let Some(event) = event_listener.recv().await {
        println!("Event received: {}", my_event_format(&event));
    }
}
fn my_event_format(event: &Event) -> String {
    format!("{} ({}) just started.", event.name, event.description)
}
