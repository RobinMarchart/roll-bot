use bot_utils::client_utils::EvaluationErrors;
use serenity::{client::Context, model::channel::Message};

pub(crate) async fn roll(
    context: &Context,
    message: Message,
    rolls: Vec<(Result<Vec<(i64, Vec<i64>)>, EvaluationErrors>, String)>,
    extended_info: bool,
) {
    for (res, expr) in rolls {
        if let Err(err) = message
            .channel_id
            .send_message(context, |m| {
                match res {
                    Ok(r) => {
                        m.content(format!(
                            "{} => [{}]",
                            expr,
                            r.iter()
                                .map(|result| format!("`{}`", result.0))
                                .reduce(|r1, r2| format!("{}, {}", r1, r2))
                                .unwrap_or(" ".to_string())
                        ));
                        if extended_info
                            && r.len() < 11
                            && r.get(0).map_or(false, |r| r.1.len() < 21)
                        {
                            m.embed(|e| {
                                e.description(
                                    r.iter()
                                        .map(|r| {
                                            format!(
                                                "[{}]",
                                                r.1.iter()
                                                    .map(|r| format!("`{}`", r))
                                                    .reduce(|r1, r2| format!("{}, {}", r1, r2))
                                                    .unwrap_or(" ".to_string())
                                            )
                                        })
                                        .reduce(|r1, r2| format!("{}\n{}", r1, r2))
                                        .unwrap(),
                                )
                            });
                        }
                    }
                    Err(e) => {
                        m.content(match e {
                            EvaluationErrors::DivideByZero => {
                                "*Division by 0 detected*".to_string()
                            }
                            EvaluationErrors::Timeout => "*Timeout*".to_string(),
                            EvaluationErrors::Overflow => "*Overflow detected*".to_string(),
                        });
                    }
                };
                m.reference_message(&message)
                    .allowed_mentions(|m| m.empty_users())
            })
            .await
        {
            log::warn!("unable to reply to message {}: {}", message.id, err);
        }
    }
}
