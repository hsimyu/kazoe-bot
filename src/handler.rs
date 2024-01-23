use regex::Regex;
use rusqlite::Connection;
use serenity::prelude::Context;
use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::*,
};
use std::{sync::Arc, sync::Mutex};

use crate::record::*;

pub struct Handler {
    pub db_connection: Arc<Mutex<Connection>>,
}

async fn reply_to(ctx: &Context, msg: &Message, str: &str) {
    println!("Send reply: {}", str);
    if let Err(why) = msg.reply(&ctx.http, str).await {
        println!("Error replying: {:?}", why);
    }
}

async fn register_new_pattern(handler: &Handler, ctx: &Context, msg: &Message) {
    // パターンをキャプチャ
    let re = Regex::new(r"かぞえて\s+(?<pattern>.*)$").unwrap();

    let pattern: String = match re.captures(&msg.content) {
        Some(caps) => caps["pattern"].to_string(),
        None => {
            reply_to(
                &ctx,
                &msg,
                "\"かぞえて [pattern]\" のようにお願いしてくださいヒン",
            )
            .await;
            return;
        }
    };

    println!("Captured pattern: {}", pattern);

    let record = PatternRecord {
        id: 0,
        channel_id: msg.channel_id.to_string(),
        pattern: pattern,
    };

    register_pattern(&handler.db_connection.lock().unwrap(), &record);
    reply_to(&ctx, &msg, "ヒヒーン！").await;
}

async fn try_count_new_message(handler: &Handler, ctx: &Context, msg: &Message) {
    let channel_id = msg.channel_id.to_string();

    let result = find_pattern(
        &handler.db_connection.lock().unwrap(),
        &channel_id,
        &msg.content,
    );

    match result {
        Some(pattern_record) => {
            println!("Found: pattern = {:?}", pattern_record);

            // 数字を抽出できたらそれを量として加算する
            let re = Regex::new(
                format!("(?<count>\\d+)\\s*{}", pattern_record.pattern.as_str()).as_str(),
            )
            .unwrap();

            let amount: i32 = match re.captures(&msg.content) {
                Some(caps) => caps["count"].to_string().parse().unwrap(),
                None => 1,
            };

            println!("captured amount = {}", amount);

            // 発言がマッチしていたのでカウントする
            let count = find_count(
                &handler.db_connection.lock().unwrap(),
                pattern_record.id,
                &msg.author.id.to_string(),
            );

            match count {
                Some(mut count_record) => {
                    println!("Found: count = {:?}", count_record);
                    count_record.count += amount;
                    update_count(&handler.db_connection.lock().unwrap(), &count_record);
                    reply_to(&ctx, &msg, format!("{}", count_record.count).as_str()).await;
                }
                None => {
                    register_new_count(
                        &handler.db_connection.lock().unwrap(),
                        &CountRecord {
                            id: 0,
                            pattern_id: pattern_record.id,
                            user_id: msg.author.id.to_string(),
                            count: amount,
                        },
                    );
                    reply_to(&ctx, &msg, format!("{}", amount).as_str()).await;
                }
            }
        }
        None => return, // 何もしない
    }
}

async fn try_set_count(handler: &Handler, ctx: &Context, msg: &Message) {
    let channel_id = msg.channel_id.to_string();

    let result = find_pattern(
        &handler.db_connection.lock().unwrap(),
        &channel_id,
        &msg.content,
    );

    match result {
        Some(pattern_record) => {
            println!("Found: pattern = {:?}", pattern_record);

            // パターンの後に来た数字で上書きする
            let re = Regex::new(
                format!("{}\\s+(?<count>\\d+)", pattern_record.pattern.as_str()).as_str(),
            )
            .unwrap();

            let amount: i32 = match re.captures(&msg.content) {
                Some(caps) => caps["count"].to_string().parse().unwrap(),
                None => {
                    // 失敗
                    reply_to(
                        &ctx,
                        &msg,
                        "\"うわがき [パターン] [量]\" のようにお願いしてくださいヒン",
                    )
                    .await;
                    return;
                }
            };

            println!("captured amount = {}", amount);

            // 発言がマッチしていたのでカウントする
            let count = find_count(
                &handler.db_connection.lock().unwrap(),
                pattern_record.id,
                &msg.author.id.to_string(),
            );

            match count {
                Some(mut count_record) => {
                    println!("Found: count = {:?}", count_record);
                    count_record.count = amount; // 加算ではなく上書きする
                    update_count(&handler.db_connection.lock().unwrap(), &count_record);
                    reply_to(&ctx, &msg, format!("{}", count_record.count).as_str()).await;
                }
                None => {
                    register_new_count(
                        &handler.db_connection.lock().unwrap(),
                        &CountRecord {
                            id: 0,
                            pattern_id: pattern_record.id,
                            user_id: msg.author.id.to_string(),
                            count: amount,
                        },
                    );
                    reply_to(&ctx, &msg, format!("{}", amount).as_str()).await;
                }
            }
        }
        None => return, // 何もしない
    }
}

async fn try_remove_pattern(handler: &Handler, ctx: &Context, msg: &Message) {
    let channel_id = msg.channel_id.to_string();

    let result = find_pattern(
        &handler.db_connection.lock().unwrap(),
        &channel_id,
        &msg.content,
    );

    match result {
        Some(pattern_record) => {
            println!("Found: pattern = {:?}", pattern_record);
            //発見したパターンを削除する

            delete_pattern(&handler.db_connection.lock().unwrap(), pattern_record.id);

            reply_to(
                &ctx,
                &msg,
                format!("削除しました: {}", pattern_record.pattern).as_str(),
            )
            .await;
        }
        None => return, // 何もしない
    }
}

fn is_mention_to_bot(ctx: &Context, msg: &Message) -> bool {
    // NOTE: msg.mentions_me() に ctx.cache をうまく渡せないので使用していない
    let bot_id = ctx.cache.current_user().id;
    let mentions = msg.mentions.clone();
    return mentions.iter().any(|mention| mention.id == bot_id);
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if !msg.author.bot {
            println!("Message: {}", msg.content);

            // bot 自身へのメンションならパターン登録/編集する
            if is_mention_to_bot(&ctx, &msg) {
                if msg.content.contains("かぞえて") {
                    register_new_pattern(&self, &ctx, &msg).await;
                } else if msg.content.contains("うわがき") {
                    // 設定値を更新する
                    try_set_count(&self, &ctx, &msg).await;
                } else if msg.content.contains("けして") {
                    // 設定値を更新する
                    try_remove_pattern(&self, &ctx, &msg).await;
                }
            } else {
                // そうではないので、パターン一致かどうか確認する
                try_count_new_message(&self, &ctx, &msg).await;
            }
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}
