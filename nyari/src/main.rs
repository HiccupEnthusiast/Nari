pub mod commands;

use std::env;

use serenity::{
    async_trait,
    framework::{standard::macros::group, StandardFramework},
    http::Http,
    model::prelude::Ready,
    prelude::{Context, EventHandler, GatewayIntents},
    Client,
};

use crate::commands::status::HELLO_COMMAND;
use crate::commands::status::PING_COMMAND;

#[group]
#[commands(ping, hello)]
struct General;
struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, r: Ready) {
        println!("Connected as: {}#{}", r.user.name, r.user.discriminator)
    }
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().expect("Failed to load .env file");
    let token = env::var("DISCORD_TOKEN").expect("Expected DISCORD_TOKEN in environment variables");

    let http = Http::new(&token);

    let framework = StandardFramework::new()
        .configure(|c| c.prefix("n!"))
        .group(&GENERAL_GROUP);

    let intents = GatewayIntents::all();

    let mut bot = Client::builder(&token, intents)
        .framework(framework)
        .event_handler(Handler)
        .await
        .expect("Error while creating the bot");

    if let Err(e) = bot.start().await {
        println!("Error while starting the bot: {:?}", e);
    }
}
