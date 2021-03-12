use crate::storage::{ClientId, StorageHandle};
use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case},
    character::complete::{multispace0, multispace1, satisfy},
    combinator::{eof, map, recognize, success},
    multi::many1,
    sequence::{pair, preceded, terminated},
    IResult,
};
use robins_dice_roll::{dice_types::Expression, parser::roll};
use std::sync::Arc;
use unicode_xid::UnicodeXID;

#[derive(Debug, Clone)]
pub enum Command {
    Help,
    RollHelp,
    Info,
    SetCommandPrefix(String),
    GetCommandPrefix,
    AddRollPrefix(String),
    RemoveRollPrefix(String),
    ListRollPrefix,
    AddAlias(String, Expression),
    RemoveAlias(String),
    ListAlias,
    AliasRoll(Arc<Expression>),
    Roll(Expression),
}

fn chars_set(input: &str) -> IResult<&str, char> {
    satisfy(|c| UnicodeXID::is_xid_start(c))(input)
}

fn parse_help(input: &str) -> IResult<&str, Command> {
    map(alt((tag_no_case("help"), tag_no_case("h"))), |_| {
        Command::Help
    })(input)
}

fn parse_command_prefix(input: &str) -> IResult<&str, Command> {
    preceded(
        terminated(
            alt((
                tag_no_case("command_prefix"),
                tag_no_case("command-prefix"),
                tag_no_case("cp"),
            )),
            multispace1,
        ),
        alt((
            map(alt((tag_no_case("get"), tag_no_case("g"))), |_| {
                Command::GetCommandPrefix
            }),
            map(
                preceded(
                    pair(alt((tag_no_case("set"), tag_no_case("s"))), multispace1),
                    recognize(many1(chars_set)),
                ),
                |s| Command::SetCommandPrefix(s.to_owned()),
            ),
        )),
    )(input)
}

fn parse_roll_help(input: &str) -> IResult<&str, Command> {
    map(
        alt((
            tag_no_case("roll-help"),
            tag_no_case("roll_help"),
            tag_no_case("rh"),
        )),
        |_| Command::RollHelp,
    )(input)
}

fn parse_info(input: &str) -> IResult<&str, Command> {
    map(alt((tag_no_case("info"), tag_no_case("i"))), |_| {
        Command::Info
    })(input)
}

fn parse_roll_prefix(input: &str) -> IResult<&str, Command> {
    preceded(
        terminated(
            alt((
                tag_no_case("roll-prefix"),
                tag_no_case("roll_prefix"),
                tag_no_case("rp"),
            )),
            multispace1,
        ),
        alt((
            map(alt((tag_no_case("list"), tag_no_case("l"))), |_| {
                Command::ListRollPrefix
            }),
            map(
                preceded(
                    pair(alt((tag_no_case("set"), tag_no_case("s"))), multispace1),
                    recognize(many1(chars_set)),
                ),
                |s| Command::AddRollPrefix(s.to_owned()),
            ),
            map(
                preceded(
                    pair(alt((tag_no_case("remove"), tag_no_case("r"))), multispace1),
                    recognize(many1(chars_set)),
                ),
                |s| Command::RemoveRollPrefix(s.to_owned()),
            ),
        )),
    )(input)
}

fn parse_roll_command(input: &str) -> IResult<&str, Command> {
    preceded(
        pair(alt((tag_no_case("roll"), tag_no_case("r"))), multispace1),
        map(roll::parse, |e| Command::Roll(e)),
    )(input)
}

fn parse_alias(input: &str) -> IResult<&str, Command> {
    preceded(
        pair(alt((tag_no_case("alias"), tag_no_case("a"))), multispace1),
        alt((
            preceded(
                pair(alt((tag_no_case("add"), tag_no_case("a"))), multispace1),
                map(
                    pair(
                        terminated(recognize(many1(chars_set)), multispace1),
                        roll::parse,
                    ),
                    |(alias, expr)| Command::AddAlias(alias.to_owned(), expr),
                ),
            ),
            preceded(
                pair(alt((tag_no_case("remove"), tag_no_case("r"))), multispace1),
                map(recognize(many1(chars_set)), |alias| {
                    Command::RemoveAlias(alias.to_owned())
                }),
            ),
            map(alt((tag_no_case("list"), tag_no_case("l"))), |_| {
                Command::ListAlias
            }),
        )),
    )(input)
}

fn parse_command<'a>(input: &'a str, prefix: &str) -> IResult<&'a str, Command> {
    terminated(
        preceded(
            pair(tag(prefix), multispace1),
            alt((
                parse_help,
                parse_roll_help,
                parse_info,
                parse_command_prefix,
                parse_roll_prefix,
                parse_alias,
                parse_roll_command,
                success(Command::Help),
            )),
        ),
        pair(multispace0, eof),
    )(input)
}

fn parse_roll<'a>(input: &'a str, prefix: &str) -> IResult<&'a str, Command> {
    map(preceded(pair(tag(prefix), multispace0), roll::parse), |e| {
        Command::Roll(e)
    })(input)
}

pub async fn parse<Id: ClientId>(
    string: &str,
    id: Id,
    store: &StorageHandle<Id>,
) -> Option<Command> {
    if let Ok((_, c)) = parse_command(string, &store.get_command_prefix(id.clone()).await) {
        Some(c)
    } else if let Some(command) = store
        .get_roll_prefixes(id.clone())
        .await
        .iter()
        .map(|prefix| parse_roll(string, prefix))
        .find_map(|r| r.ok().map(|res| res.1))
    {
        Some(command)
    } else {
        store
            .get_alias(id.clone(), string.trim().to_owned())
            .await
            .map(|e| Command::AliasRoll(e))
    }
}
