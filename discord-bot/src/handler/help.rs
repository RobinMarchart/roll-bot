pub(crate) async fn help(
    context: serenity::client::Context,
    message: serenity::model::channel::Message,
) {
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
