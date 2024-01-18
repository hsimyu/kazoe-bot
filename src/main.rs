use dotenv::dotenv;
use regex::Regex;
use rusqlite::{Connection, Result};
use serenity::{
    all::{GatewayIntents, User},
    async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::*,
};
use std::{env, sync::Arc, sync::Mutex};
use tokio;

#[derive(Debug)]
struct PatternRecord {
    id: i32,
    channel_id: String,
    pattern: String,
}

struct Handler {
    db_connection: Arc<Mutex<Connection>>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if !msg.author.bot {
            println!("Context: {:?}", ctx);
            println!();
            println!("Message: {}", msg.content);
            println!("{:?}", msg);
            println!();

            for mentioned_user in msg.mentions.iter() {
                println!(
                    "- mentioned: name = {}, id = {}",
                    mentioned_user.name, mentioned_user.id
                );
            }

            if msg.content.contains("かぞえて") {
                if let Err(why) = msg.reply(&ctx.http, "承知しました！").await {
                    println!("Error replying: {:?}", why);
                }

                // パターンをキャプチャ
                let re = Regex::new(r"かぞえて (?<pattern>.*)$").unwrap();
                let Some(caps) = re.captures(&msg.content) else {
                    println!("no match!");
                    return;
                };

                println!("Captured pattern: {}", &caps["pattern"]);

                let record = PatternRecord {
                    id: 0,
                    channel_id: msg.channel_id.to_string(),
                    pattern: caps["pattern"].to_string(),
                };

                register_pattern(&self.db_connection.lock().unwrap(), &record);
                dump_pattern_record(&self.db_connection.lock().unwrap());
            } else {
                // 登録依頼ではないのでパターンを検索する
                let channel_id = msg.channel_id.to_string();
                find_pattern(&self.db_connection.lock().unwrap(), &channel_id);
            }
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

fn create_table(conn: &Connection) {
    conn.execute(
        "CREATE TABLE pattern_record (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            channel_id TEXT NOT NULL,
            pattern TEXT NOT NULL
        )",
        (),
    )
    .expect("Failed to create pattern_record");
}

fn register_pattern(conn: &Connection, record: &PatternRecord) {
    conn.execute(
        "INSERT INTO pattern_record (id, channel_id, pattern) VALUES (?1, ?2, ?3)",
        (&record.id, &record.channel_id, &record.pattern),
    )
    .expect("Failed to insert message");
}

fn dump_pattern_record(conn: &Connection) {
    let mut stmt = conn
        .prepare("SELECT id, channel_id, pattern FROM pattern_record")
        .unwrap();

    let iter = stmt
        .query_map([], |row| {
            Ok(PatternRecord {
                id: row.get(0)?,
                channel_id: row.get(1)?,
                pattern: row.get(2)?,
            })
        })
        .unwrap();

    for mes in iter {
        println!("Pattern: {:?}", mes.unwrap());
    }
}

fn find_pattern(conn: &Connection, channel_id: &String) {
    let mut stmt = conn
        .prepare("SELECT * FROM pattern_record WHERE channel_id = ?1")
        .unwrap();

    let iter = stmt
        .query_map([&channel_id], |row| {
            Ok(PatternRecord {
                id: row.get(0)?,
                channel_id: row.get(1)?,
                pattern: row.get(2)?,
            })
        })
        .unwrap();

    for mes in iter {
        println!("Pattern: {:?}", mes.unwrap());
    }
}

#[tokio::main]
async fn main() {
    let conn = Connection::open_in_memory().expect("Failed to open memory");
    create_table(&conn);

    dotenv().ok();

    let conn = Arc::new(Mutex::new(conn));

    let bot_token = env::var("DISCORD_BOT_TOKEN").expect("DISCORD_BOT_TOKEN must be set in .env");
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(bot_token, intents)
        .event_handler(Handler {
            db_connection: Arc::clone(&conn),
        })
        .await
        .expect("Error creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
