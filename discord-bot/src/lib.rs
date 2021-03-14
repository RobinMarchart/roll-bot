use bot_utils::{Bot, ClientUtils, CommandResult, GlobalUtils};

use serenity::{
    client::ClientBuilder,
    model::{
        channel::Message,
        event::Event,
        id::{GuildId, UserId},
    },
    prelude::RawEventHandler,
};

use async_trait::async_trait;
use std::sync::Arc;

pub struct DiscordBot {}

impl DiscordBot {
    pub fn new() -> Box<DiscordBot> {
        Box::new(DiscordBot {})
    }
}

struct DiscordBotHandler {
    guild_utils: ClientUtils<GuildId>,
    dm_utils: ClientUtils<UserId>,
}

trait FoldFirstIterator: Iterator {
    fn fold_first<F: std::ops::FnMut(Self::Item, Self::Item) -> Self::Item>(
        self,
        f: F,
    ) -> Option<Self::Item>;
}

impl<I: Iterator + Sized> FoldFirstIterator for I {
    fn fold_first<F: std::ops::FnMut(Self::Item, Self::Item) -> Self::Item>(
        self,
        mut f: F,
    ) -> Option<Self::Item> {
        self.fold(None, |a, b| match a {
            Some(v) => Some(f(v, b)),
            None => Some(b),
        })
    }
}

#[async_trait]
impl Bot for DiscordBot {
    async fn run(&self, utils: Arc<GlobalUtils>) {
        let token = std::env::var("DISCORD_TOKEN")
            .expect("No Discord API Token provided in DISCORD_TOKEN env var");
        let handler = DiscordBotHandler {
            guild_utils: ClientUtils::new(utils.clone(), "discord_guilds")
                .await
                .expect("error creating guild storage"),
            dm_utils: ClientUtils::new(utils, "discord_dm")
                .await
                .expect("error creating dm storage"),
        };

        let mut client = ClientBuilder::new(token)
            .raw_event_handler(handler)
            .await
            .expect("Error creating Client");
        log::info!("created client");
        client.start().await.expect("Client error")
    }
}

async fn check_priviledged_access(context: &serenity::client::Context, message: &Message) -> bool {
    match message.guild_id {
        Some(guild) => match guild.to_partial_guild(&context).await {
            Ok(g) => {
                if g.owner_id == message.author.id {
                    true
                } else {
                    match g.member(&context, message.author.id.clone()).await {
                        Ok(member) => {
                            for roll in member
                                .roles
                                .iter()
                                .map(|id| g.roles.get(id))
                                .filter_map(|x| x)
                            {
                                if roll.permissions.administrator() {
                                    return true;
                                }
                            }
                            false
                        }
                        Err(err) => {
                            log::warn!("unable to get member {}: {}", &message.author.id, err);
                            false
                        }
                    }
                }
            }
            Err(err) => {
                log::warn!("unable to retrieve guild {}: {}", &guild, err);
                false
            }
        },
        None => true, //user is always allowed to run every command in dm channels
    }
}

async fn respond(
    context: serenity::client::Context,
    message: serenity::model::channel::Message,
    response: CommandResult,
) {
    match response {
        CommandResult::Help => {
            if let Err(err) = message.channel_id.send_message(&context, |m| {
                m.reference_message((message.channel_id.clone(),message.id.clone()))
                    .allowed_mentions(|mentions|mentions.empty_users())
                 .embed(|e|{
                     e.title("Command Syntax")
                      .description("
all Commands are prefixed with `{command-prefix}`, which defaults to rrb!.
The prefix is recognized both with or without following whitespace.
Both tab and newline are recognized as whitespace. Several whitespace characters are also accepted.
                            ").field("Privileged Commands", "
Some commands require special permissions to use. They are prefixed with \\* in this overview.
", false)
                      .field(
                          "General Help Commands",
                          "
`help`, `h` => show this help text
`roll-help`, `roll_help`, `rh` => show help on roll syntax
`info`, `i` => show extra info about this Bot
",
                          false
                      ).field(
                          "Command Prefix",
                          "
The Command Prefix group always begins with `command-prefix`, `command_prefix` or `cp` followed by whitespace. The commands in this group are:

\\* `set [prefix]` , `s [prefix]` => set command prefix to `[prefix]`. Whitespace Characters are not allowed in `[prefix]`.
`get`, `g` => get command prefix.
",
                          false
                      ).field(
                          "Roll",
                          "
`roll [roll-statement]`, `r [roll-statement]` => roll dice as described in `[roll-statement]`. See roll-help for Information on the grammar for this.
",
                          false
                      ).field(
                          "Roll Prefix",
                          "
The Roll Prefix group always begins with `roll-prefix`, `roll_prefix` or `rp` followed by whitespace.
This is an alternate Way to roll `[roll statement]`s. Just append your `[roll statement]` to one of these.

\\* `add [prefix]`, `a [prefix]` => add `[prefix]` to the list of roll prefixes.
\\* `remove [prefix]`, `r [prefix]` => remove `[prefix]` from the list of roll prefixes.
`list`, `l` => list roll prefixes on this Server
",
                          false
                      ).field(
                          "Alias",
                          "
The Alias group always begins with `alias` or `a`, followed by whitespace.
This allows to specify messages, which will be interpreted as roll statements, if they are the only content of the message.
One usage of this is to enable saving roll statements like 6{4d6k3}, the statement used to roll for stats in D&D

\\* `add [alias] [roll statement]`, `a [alias] [roll statement]` => Adds `[alias]` as an alias for `[roll statement]`.
\\* `remove [alias]`, `r [alias]` => remove `[alias]` from known aliases.
`list`, `l` => list known aliases.
",
                          false
                      ).field("About This Bot", "The source code for this Bot is available on [GitHub](https://github.com/RobinMarchart/roll-bot)", false)
                 })
            }).await {
                log::warn!("Unable to reply to message {}: {}",message.id,err)
            }
        }
        CommandResult::RollHelp => {}
        CommandResult::Info => {}
        CommandResult::SetCommandPrefix(prefix) => {
            if let Err(err) = Message::react(&message, &context, '✅').await {
                log::warn!("unable to react to message {}: {}", message.id, err)
            }
            if let Some(guild) = message.guild_id {
                let mut nickname = format!("[{}] Robins Roll Bot", &prefix);
                if nickname.len() > 32 {
                    nickname = format!("[{}] Roll Bot", &prefix);
                    if nickname.len() > 32 {
                        nickname = format!("[{}] Roll", &prefix);
                        if nickname.len() > 32 {
                            nickname = format!("[{}]", &prefix);
                            if nickname.len() > 32 {
                                nickname = "Robins Roll Bot".to_string();
                            }
                        }
                    }
                }
                if let Some(err) = guild.edit_nickname(&context, Some(&nickname)).await.err() {
                    log::warn!("Unable to change nickname in {}: {}", guild, err);
                }
            }
        }
        CommandResult::GetCommandPrefix(prefix) => {
            if let Err(err) = Message::react(&message, &context, '✅').await {
                log::warn!("unable to react to message {}: {}", message.id, err)
            }
            if let Err(err) = Message::reply(&message, &context, format!("`{}`", prefix)).await {
                log::warn!("Unable to reply to message {}: {}", message.id, err)
            }
        }
        CommandResult::AddRollPrefix(result) => {
            if let Err(err) = Message::react(
                &message,
                &context,
                match result {
                    Ok(_) => '✅',
                    Err(_) => '❌',
                },
            )
            .await
            {
                log::warn!("unable to react to message {}: {}", message.id, err)
            }
        }
        CommandResult::RemoveRollPrefix(result) => {
            if let Err(err) = Message::react(
                &message,
                &context,
                match result {
                    Ok(_) => '✅',
                    Err(_) => '❌',
                },
            )
            .await
            {
                log::warn!("unable to react to message {}: {}", message.id, err)
            }
        }
        CommandResult::ListRollPrefix(prefixes) => {
            if let Some(m) = prefixes
                .iter()
                .map(|p| format!("`{}`", p))
                .fold_first(|p1, p2| format!("{}\n{}", p1, p2))
            {
                if let Err(err) = Message::reply(&message, &context, m).await {
                    log::warn!("Unable to reply to message: {}", err)
                }
            }
        }
        CommandResult::AddAlias => {
            if let Err(err) = Message::react(&message, &context, '✅').await {
                log::warn!("unable to react to message {}: {}", message.id, err)
            }
        }
        CommandResult::RemoveAlias(result) => {
            if let Err(err) = Message::react(
                &message,
                &context,
                match result {
                    Ok(_) => '✅',
                    Err(_) => '❌',
                },
            )
            .await
            {
                log::warn!("unable to react to message {}: {}", message.id, err)
            }
        }
        CommandResult::ListAlias(alias) => {
            if let Some(m) = alias
                .iter()
                .map(|(alias, expr)| format!("`{}` => `{}`", alias, expr))
                .fold_first(|p1, p2| format!("{}\n{}", p1, p2))
            {
                if let Err(err) = Message::reply(&message, &context, m).await {
                    log::warn!("Unable to reply to message: {}", err)
                }
            }
        }
        CommandResult::Roll(res, expr) => {
            let mes = match res {
                Ok(r) => {
                    format!(
                        "{} => [{}]",
                        expr,
                        r.iter()
                            .map(|result| format!("`{}`", result.0))
                            .fold_first(|r1, r2| format!("{}, {}", r1, r2))
                            .unwrap_or(" ".to_string())
                    )
                }
                Err(e) => match e {
                    bot_utils::EvaluationErrors::DivideByZero => {
                        "*Division by 0 detected*".to_string()
                    }
                    bot_utils::EvaluationErrors::Timeout => "*Timeout*".to_string(),
                    bot_utils::EvaluationErrors::Overflow => "*Overflow detected*".to_string(),
                },
            };
            if let Err(err) = Message::reply(&message, &context, mes).await {
                log::warn!("Unable to reply to message: {}", err)
            }
        },
        CommandResult::InsufficentPermission=>{
            if let Err(err)=Message::react(&message,&context, '❌').await{
                log::warn!("unable to add reaction to message {}: {}",message.id,err);
            }
        }
    }
}

#[async_trait]
impl RawEventHandler for DiscordBotHandler {
    async fn raw_event(&self, ctx: serenity::client::Context, ev: Event) {
        match ev {
            Event::MessageCreate(event) => {
                if event.message.author.bot {
                } else if let Some(guild) = event.message.guild_id {
                    if let Some(response) = self
                        .guild_utils
                        .eval(guild.clone(), &event.message.content, || {
                            check_priviledged_access(&ctx, &event.message)
                        })
                        .await
                    {
                        respond(ctx, event.message, response).await;
                    }
                } else {
                    if let Some(response) = self
                        .dm_utils
                        .eval(
                            event.message.author.id.clone(),
                            &event.message.content,
                            || check_priviledged_access(&ctx, &event.message),
                        )
                        .await
                    {
                        respond(ctx, event.message, response).await;
                    }
                }
            }
            _ => {}
        }
    }
}
