use serenity::{client::Context, model::channel::Message};

pub(crate) async fn add_alias(context: Context, message: Message) {
    if let Err(err) = Message::react(&message, &context, '✅').await {
        log::warn!("unable to react to message {}: {}", message.id, err)
    }
}

pub(crate) async fn remove_alias(context: Context, message: Message, result: Result<(), ()>) {
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

pub(crate) async fn list_aliases(
    context: Context,
    message: Message,
    aliases: Vec<(String, String)>,
) {
    if let Some(m) = aliases
        .iter()
        .map(|(alias, expr)| format!("`{}` => `{}`", alias, expr))
        .reduce(|p1, p2| format!("{}\n{}", p1, p2))
    {
        if let Err(err) = Message::reply(&message, &context, m).await {
            log::warn!("Unable to reply to message: {}", err)
        }
    }
}
