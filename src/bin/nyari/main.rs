use std::{env, sync::Arc};

use twilight_cache_inmemory::{InMemoryCache, ResourceType};
use twilight_gateway::{Event, Shard, ShardId};
use twilight_http::Client as HttpClient;
use twilight_model::gateway::Intents;

#[tokio::main]
async fn main() {
    dotenv::dotenv().expect("Failed to load .env file");
    tracing_subscriber::fmt::init();

    let token = env::var("DISCORD_TOKEN").expect("Expected DISCORD_TOKEN in environment variables");

    // Use intents to only receive guild message events.
    let mut shard = Shard::new(
        ShardId::ONE,
        token.clone(),
        Intents::GUILD_MESSAGES | Intents::MESSAGE_CONTENT,
    );

    // HTTP is separate from the gateway, so create a new client.
    let http = Arc::new(HttpClient::new(token));

    // Since we only care about new messages, make the cache only
    // cache new messages.
    let cache = InMemoryCache::builder()
        .resource_types(ResourceType::MESSAGE)
        .build();

    // Process each event as they come in.
    loop {
        let event = match shard.next_event().await {
            Ok(event) => event,
            Err(source) => {
                println!("error receiving event {source}");

                if source.is_fatal() {
                    break;
                }

                continue;
            }
        };

        // Update the cache with the event.
        cache.update(&event);

        tokio::spawn(handle_event(event, Arc::clone(&http)));
    }
}

async fn handle_event(
    event: Event,
    http: Arc<HttpClient>,
) {
    match event {
        Event::Ready(r) => {
            println!("Connected as: {}#{}", r.user.name, r.user.discriminator)
        }
        Event::MessageCreate(msg) => {
            println!("{}: {}", msg.author.name, msg.content)
        }
        // Other events here...
        _ => {}
    }
}
