#[macro_use]
extern crate log;
#[macro_use]
extern crate sqlx;

mod standard;

use std::{collections::HashMap, env, mem::transmute};

use poise::{
    serenity_prelude::{self as serenity, futures::Stream, ClientBuilder, GatewayIntents},
    CreateReply, Framework, FrameworkOptions,
};
use sqlx::PgPool;
use standard::{load_all_stds, Standard};
use tokio::sync::Mutex;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;
struct Data {
    pub stds_cache: Mutex<UserStdsCache>,
    pub pool: PgPool,
    pub standards: Vec<(&'static str, Standard<'static>)>,
}

#[derive(Default)]
struct UserStdsCache {
    of: HashMap<serenity::UserId, Vec<&'static str>>,
}
impl UserStdsCache {
    pub async fn get(
        &mut self,
        pool: &PgPool,
        id: serenity::UserId,
        stds: &[(&'static str, Standard<'static>)],
    ) -> Result<&[&'static str], Error> {
        if let Some(x) = unsafe {
            // This is necessary exclusively cuz Rust doesn't like this.
            transmute::<
                &mut HashMap<serenity::UserId, Vec<&'static str>>,
                &'static mut HashMap<serenity::UserId, Vec<&'static str>>,
            >(&mut self.of)
        }
        .get(&id)
        {
            Ok(x.as_slice())
        } else {
            if self.of.len() > 100 {
                let key = *self.of.keys().next().unwrap();
                self.of.remove(&key);
            }
            let Some(resp) = query!("select stds from stds where uid=$1", unsafe {
                transmute::<u64, i64>(id.get())
            })
            .fetch_optional(pool)
            .await?
            else {
                return Ok(self.of.entry(id).or_insert(vec!["core"]));
            };
            let mut vec = vec![];
            for x in resp.stds {
                if let Some(x) = stds.iter().map(|x| x.0).find(|y| *y == x) {
                    vec.push(x);
                }
            }
            Ok(self.of.entry(id).or_insert(vec))
        }
    }

    pub async fn update(
        &mut self,
        pool: &PgPool,
        id: serenity::UserId,
        to: Vec<&'static str>,
    ) -> Result<(), Error> {
        query!(
            "insert into stds values($1, $2) on conflict(uid) do update set stds = excluded.stds",
            unsafe { transmute::<u64, i64>(id.get()) },
            &to.iter().map(|x| x.to_string()).collect::<Vec<String>>()
        )
        .execute(pool)
        .await?;
        if !self.of.contains_key(&id) && self.of.len() > 100 {
            let key = *self.of.keys().next().unwrap();
            self.of.remove(&key);
        }
        self.of.insert(id, to);
        Ok(())
    }

    pub async fn delete(&mut self, pool: &PgPool, id: serenity::UserId) -> Result<(), Error> {
        query!("delete from stds where uid=$1", unsafe {
            transmute::<u64, i64>(id.get())
        })
        .execute(pool)
        .await?;
        self.of.remove(&id);
        Ok(())
    }
}

async fn report_on(str: &str, data: &Data, user: serenity::UserId) -> Result<String, Error> {
    let candidate = str.split(|x: char| x.is_whitespace()).rev();
    let mut report = String::new();

    let mut stds = data.stds_cache.lock().await;
    let stds = stds
        .get(&data.pool, user, data.standards.as_slice())
        .await?;

    for candidate in candidate {
        if candidate.is_empty() {
            continue;
        }
        if !candidate.starts_with('/') {
            break;
        }

        if let Some(x) = data
            .standards
            .iter()
            .filter(|x| stds.contains(&x.0))
            .flat_map(|x| x.1.tags.get(candidate).map(|y| (candidate, y)))
            .next()
        {
            if !report.is_empty() {
                report.push('\n');
            }

            report.push_str("**");
            report.push_str(x.0);
            report.push_str("**: ");
            report.push_str(x.1.trim());
        }
    }

    if report.is_empty() {
        report = "*No tone tags were found*".to_string();
    }

    Ok(report)
}

async fn send_long(ctx: &Context<'_>, text: &str, ephemeral: bool) -> Result<(), Error> {
    let mut str = String::with_capacity(2000);

    // It is assumed that one line cannot be 2000+ symbols long
    for line in text.lines() {
        if str.len() + line.len() >= 1999 {
            let mut s = String::with_capacity(2000);
            std::mem::swap(&mut s, &mut str);
            ctx.send(CreateReply {
                content: Some(s),
                allowed_mentions: Some(Default::default()),
                ephemeral: Some(ephemeral),
                ..Default::default()
            })
            .await?;
        }

        if !str.is_empty() {
            str.push('\n');
        }

        str += line;
    }

    if !str.is_empty() {
        ctx.send(CreateReply {
            content: Some(str),
            allowed_mentions: Some(Default::default()),
            ephemeral: Some(ephemeral),
            ..Default::default()
        })
        .await?;
    }

    Ok(())
}

/// See tone tags on a message
#[poise::command(context_menu_command = "Tone tags")]
async fn detect_msg(
    ctx: Context<'_>,
    #[description = "Message to check tone tags of"] link: serenity::Message,
) -> Result<(), Error> {
    send_long(
        &ctx,
        &report_on(&link.content, ctx.data(), ctx.author().id).await?,
        true,
    )
    .await?;
    Ok(())
}

/// See tone tags on a message (non-ephimeral)
#[poise::command(context_menu_command = "Tone tags (non-ephemeral)")]
async fn detect_msg_neph(
    ctx: Context<'_>,
    #[description = "Message to check tone tags of"] link: serenity::Message,
) -> Result<(), Error> {
    send_long(
        &ctx,
        &report_on(&link.content, ctx.data(), ctx.author().id).await?,
        false,
    )
    .await?;
    Ok(())
}

/// See tone tags on a message (requires Tone Tags to be added as a bot)
#[poise::command(slash_command, rename = "msg")]
async fn explain_msg(
    ctx: Context<'_>,
    #[description = "Message to check tone tags of"] link: serenity::Message,
    #[description = "Whether message should be ephemeral"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    send_long(
        &ctx,
        &report_on(&link.content, ctx.data(), ctx.author().id).await?,
        ephemeral.unwrap_or(true),
    )
    .await?;
    Ok(())
}

/// See tone tags on provided text
#[poise::command(slash_command, rename = "txt")]
async fn explain_txt(
    ctx: Context<'_>,
    #[description = "Tags to check"] tags: String,
    #[description = "Whether message should be ephemeral"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    send_long(
        &ctx,
        &report_on(&tags, ctx.data(), ctx.author().id).await?,
        ephemeral.unwrap_or(true),
    )
    .await?;
    Ok(())
}

/// See tone tags
#[poise::command(
    slash_command,
    subcommands("explain_txt", "explain_msg"),
    subcommand_required
)]
async fn explain(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

async fn standards_autocomplete<'a>(
    ctx: Context<'_>,
    partial: &'a str,
) -> impl Stream<Item = String> + 'a {
    let parts_og: Vec<&str> = partial.split(',').collect();

    let mut prefixx: Vec<&str> = vec![];
    for x in parts_og
        .iter()
        .take(parts_og.len().checked_sub(1).unwrap_or_default())
        .filter(|x| ctx.data().standards.iter().any(|y| y.0 == **x))
    {
        if !prefixx.contains(x) {
            prefixx.push(x);
        }
    }
    let prefix = {
        let mut str = String::new();
        for x in prefixx.iter() {
            if !str.is_empty() {
                str.push(',');
            }
            str.push_str(x);
        }
        if !str.is_empty() {
            str.push(',');
        }
        str
    };

    let search = parts_og.last().unwrap();
    let mut complete = vec![];
    for x in ctx.data().standards.iter() {
        if prefixx.contains(&x.0) {
            continue;
        }

        let mut search_id = x.0.chars();
        let mut search_with = search.chars();

        if search_with.all(|x| search_id.any(|y| y == x)) {
            complete.push(format!("{prefix}{}", x.0));
        }
    }

    futures::stream::iter(complete)
}

/// Set standards to evaluate tone tags with
#[poise::command(slash_command, rename = "set")]
async fn standards_set(
    ctx: Context<'_>,
    #[description = "Comma-separated list of standards to have enabled"]
    #[autocomplete = "standards_autocomplete"]
    standards: String,
) -> Result<(), Error> {
    let mut v: Vec<&'static str> = vec![];
    for x in standards
        .split(',')
        .filter_map(|x| ctx.data().standards.iter().find(|y| y.0 == x).map(|x| x.0))
    {
        if !v.contains(&x) {
            v.push(x);
        }
    }

    let count = v.len();

    let mut stds = ctx.data().stds_cache.lock().await;
    stds.update(&ctx.data().pool, ctx.author().id, v).await?;

    if count == 0 {
        send_long(
            &ctx,
            "*Disabled all standards. Bot is now effectively disabled*",
            true,
        )
        .await?;
    } else if count == 1 {
        send_long(&ctx, "*Enabled 1 standard*", true).await?;
    } else {
        send_long(&ctx, &format!("*Enabled {count} standards*"), true).await?;
    }

    Ok(())
}

/// View all standards
#[poise::command(slash_command, rename = "get")]
async fn standards_get(
    ctx: Context<'_>,
    #[description = "Show disabled standards"] show_disabled: Option<bool>,
    #[description = "Send output as ephemeral"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    let mut str = String::new();

    let show_disabled = show_disabled.unwrap_or(false);
    let ephemeral = ephemeral.unwrap_or(true);

    let mut filter: Vec<&'static str> = vec![];
    let mut stds = ctx.data().stds_cache.lock().await;
    let stds = stds
        .get(&ctx.data().pool, ctx.author().id, &ctx.data().standards)
        .await?;
    if stds.is_empty() && !show_disabled {
        send_long(&ctx, "*No standards are enabled*", ephemeral).await?;
        return Ok(());
    }
    filter.extend(stds);

    for x in ctx.data().standards.iter() {
        if !show_disabled && !filter.contains(&x.0) {
            continue;
        }

        if !str.is_empty() {
            str.push('\n');
        }

        str.push_str("## ");
        str.push_str(x.1.title);
        str.push_str("\n`");
        str.push_str(x.0);
        if show_disabled && filter.contains(&x.0) {
            str.push_str("` *(enabled)*\n");
        } else {
            str.push_str("`\n");
        }
        str.push_str(x.1.description.trim());
    }

    send_long(&ctx, &str, ephemeral).await?;

    Ok(())
}

/// Manage tone tag standards
#[poise::command(
    slash_command,
    subcommands("standards_get", "standards_set"),
    subcommand_required
)]
async fn standards(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Delete all of your data
#[poise::command(slash_command)]
async fn nuke_all_data(ctx: Context<'_>) -> Result<(), Error> {
    ctx.data()
        .stds_cache
        .lock()
        .await
        .delete(&ctx.data().pool, ctx.author().id)
        .await?;
    send_long(&ctx, "*Deleted all data.*", true).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenvy::dotenv().ok();
    pretty_env_logger::init();

    let standardss = load_all_stds()?;

    let token = env::var("TOKEN").expect("'TOKEN' is not set");
    let database = env::var("DATABASE_URL").expect("'DATABASE_URL' is not set");

    let database = PgPool::connect(&database).await?;
    info!("Connected to the database!");

    let fw = Framework::builder()
        .options(FrameworkOptions {
            commands: vec![
                detect_msg(),
                detect_msg_neph(),
                explain(),
                standards(),
                nuke_all_data(),
            ],
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                info!("Loaded bot!");

                Ok(Data {
                    stds_cache: Mutex::new(UserStdsCache::default()),
                    pool: database,
                    standards: standardss,
                })
            })
        })
        .build();

    let intents = GatewayIntents::non_privileged();

    ClientBuilder::new(token, intents)
        .framework(fw)
        .await
        .unwrap()
        .start()
        .await
        .unwrap();

    Ok(())
}
