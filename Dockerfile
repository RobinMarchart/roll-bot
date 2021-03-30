FROM nouchka/sqlite3 AS db
ADD ./bot-utils/migrations /sql-stmts
RUN echo /sql-stmts/*/|sort|while read line;do cat $line/up.sql;done|sqlite3 /roll-bot.sqlite
FROM scratch
ADD --chown=0:0  ./target/x86_64-unknown-linux-musl/release/roll-bot /roll-bot
VOLUME /config
COPY --from=db --chown=0:0 /roll-bot.sqlite /roll-bot-db/roll-bot.sqlite
VOLUME /roll-bot-db
ENV DB_PATH=/roll-bot-db/roll-bot.sqlite
ENV RUST_LOG=info,tracing::span=warn,robins_dice_roll::dice_roll=debug,serenity::http::request=warn,serenity::gateway::shard=warn
ENV RUST_BACKTRACE=1
ENTRYPOINT [ "/roll-bot", "/config/config.toml" ]
LABEL org.opencontainers.image.source=https://github.com/RobinMarchart/roll-bot
