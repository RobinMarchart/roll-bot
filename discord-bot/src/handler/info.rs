pub(crate) async fn info(
    context: serenity::client::Context,
    message: serenity::model::channel::Message,
    invite_url: &str,
) {
    if let Err(e) = message
        .channel_id
        .send_message(&context, |m| {
            m.reference_message((message.channel_id, message.id))
                .allowed_mentions(|m| m.empty_users())
                .embed(|e| {
                    e.title("**INFO**").field(
                        "Invite",
                        format!("open {} to add this bot to your servers", invite_url),
                        false,
                    )
                })
        })
        .await
    {
        log::warn!("Unable to reply to message {}: {}", message.id, e)
    }
}
