use serenity::{client::Context, model::channel::Message};

pub(crate) async fn add_roll_prefix(context: Context, message: Message, result: Result<(), ()>) {
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

pub(crate) async fn remove_roll_prefix(context: Context, message: Message, result: Result<(), ()>) {
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

pub(crate) async fn list_roll_prefix(context: Context, message: Message, prefixes: Vec<String>) {
    if let Some(m) = prefixes
        .iter()
        .map(|p| format!("`{}`", p))
        .reduce(|p1, p2| format!("{}\n{}", p1, p2))
    {
        if let Err(err) = Message::reply(&message, &context, m).await {
            log::warn!("Unable to reply to message: {}", err)
        }
    }
}
