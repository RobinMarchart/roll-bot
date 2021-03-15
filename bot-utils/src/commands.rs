use crate::storage::{ClientId, StorageHandle};
use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case},
    character::complete::{multispace0, multispace1, satisfy},
    combinator::{eof, map, recognize, success},
    multi::many1,
    sequence::{delimited, pair, preceded, terminated},
    IResult,
};
use robins_dice_roll::{dice_types::Expression, parser};
use std::sync::Arc;
use unicode_categories::UnicodeCategories;

#[derive(Debug, Clone, PartialEq, Eq)]
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
    ListAliases,
    AliasRoll(Arc<Expression>),
    Roll(Expression),
}

fn chars_set(input: &str) -> IResult<&str, char> {
    satisfy(|c| !(c.is_separator() || c.is_other()))(input)
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
                    pair(alt((tag_no_case("add"), tag_no_case("a"))), multispace1),
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
        pair(alt((tag_no_case("roll"), tag_no_case("r"))), multispace0),
        map(parser::parse_expression, |e| Command::Roll(e)),
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
                        parser::parse_expression,
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
                Command::ListAliases
            }),
        )),
    )(input)
}

fn parse_command<'a>(input: &'a str, prefix: &str) -> IResult<&'a str, Command> {
    preceded(
        tag(prefix),
        alt((
            delimited(
                multispace0,
                alt((
                    parse_help,
                    parse_roll_help,
                    parse_info,
                    parse_command_prefix,
                    parse_roll_prefix,
                    parse_alias,
                    parse_roll_command,
                )),
                pair(multispace0, eof),
            ),
            success(Command::Help),
        )),
    )(input)
}

fn parse_roll<'a>(input: &'a str, prefix: &str) -> IResult<&'a str, Command> {
    map(
        delimited(
            pair(tag(prefix), multispace0),
            parser::parse_expression,
            pair(multispace0, eof),
        ),
        |e| Command::Roll(e),
    )(input)
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

pub async fn parse_logging<Id: ClientId>(
    string: &str,
    id: Id,
    store: &StorageHandle<Id>,
) -> Option<Command> {
    let command = parse(string, id, store).await;
    log::info!("{:?}", &command);
    command
}

#[cfg(test)]
mod tests {
    use super::*;
    use robins_dice_roll::dice_types::*;

    #[test]
    fn test_parse_command() {
        assert_eq!(
            parse_command("! roll 1", "!"),
            Ok(("", Command::Roll(Expression::Simple(Term::Constant(1)))))
        );
        assert_eq!(
            parse_command("!cp set !", "!"),
            Ok(("", Command::SetCommandPrefix("!".to_string())))
        );
        assert_eq!(
            parse_command("ü rp add ä", "ü"),
            Ok(("", Command::AddRollPrefix("ä".to_string())))
        );

        assert_eq!(
            parse_command("! r 1d4", "!"),
            Ok((
                "",
                Command::Roll(Expression::Simple(Term::DiceThrow(
                    SelectedDice::Unchanged(FilteredDice::Simple(Dice {
                        throws: 1,
                        dice: DiceType::Number(4)
                    }))
                )))
            ))
        );
    }

    #[test]
    fn test_chars_set() {
        assert_eq!(chars_set("ä"), Ok(("", 'ä')));
        assert_eq!(chars_set(":"), Ok(("", ':')));
        assert_eq!(chars_set("$"), Ok(("", '$')));
        assert_eq!(chars_set("✅"), Ok(("", '✅')));
    }
}
