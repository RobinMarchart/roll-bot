pub use super::{
    storage::{ClientId, StorageHandle},
    VersionedRollExpr,
};
use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case},
    character::complete::{multispace0, multispace1, satisfy},
    combinator::{eof, map, recognize, success},
    multi::{many0, many1},
    sequence::{delimited, pair, preceded, terminated},
    IResult,
};
use robins_dice_roll::parser;
use std::sync::Arc;
use unicode_categories::UnicodeCategories;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Help,
    RollHelp,
    Info,
    SetCommandPrefix(String),
    GetCommandPrefix,
    SetRollInfo(bool),
    GetRollInfo,
    AddRollPrefix(String),
    RemoveRollPrefix(String),
    ListRollPrefix,
    AddAlias(String, VersionedRollExpr),
    RemoveAlias(String),
    ListAliases,
    AliasRoll(Vec<Arc<VersionedRollExpr>>),
    Roll(VersionedRollExpr),
}

fn chars_set(input: &str) -> IResult<&str, char> {
    satisfy(|c| !(c == '$' || c.is_separator() || c.is_other()))(input)
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
                tag_no_case("command prefix"),
                tag_no_case("cp"),
            )),
            multispace0,
        ),
        alt((
            map(alt((tag_no_case("get"), tag_no_case("g"))), |_| {
                Command::GetCommandPrefix
            }),
            map(
                preceded(
                    pair(alt((tag_no_case("set"), tag_no_case("s"))), multispace0),
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
            tag_no_case("roll help"),
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
                tag_no_case("roll prefix"),
                tag_no_case("rp"),
            )),
            multispace0,
        ),
        alt((
            map(alt((tag_no_case("list"), tag_no_case("l"))), |_| {
                Command::ListRollPrefix
            }),
            map(
                preceded(
                    pair(alt((tag_no_case("add"), tag_no_case("a"))), multispace0),
                    recognize(many0(chars_set)),
                ),
                |s| Command::AddRollPrefix(s.to_owned()),
            ),
            map(
                preceded(
                    pair(alt((tag_no_case("remove"), tag_no_case("r"))), multispace0),
                    recognize(many0(chars_set)),
                ),
                |s| Command::RemoveRollPrefix(s.to_owned()),
            ),
        )),
    )(input)
}

fn parse_roll_command(input: &str) -> IResult<&str, Command> {
    preceded(
        pair(alt((tag_no_case("roll"), tag_no_case("r"))), multispace0),
        map(parser::parse_labeled, |e| {
            Command::Roll(VersionedRollExpr::V2(e))
        }),
    )(input)
}

fn parse_alias(input: &str) -> IResult<&str, Command> {
    preceded(
        pair(alt((tag_no_case("alias"), tag_no_case("a"))), multispace0),
        alt((
            preceded(
                pair(alt((tag_no_case("add"), tag_no_case("a"))), multispace0),
                map(
                    pair(
                        terminated(recognize(many1(chars_set)), multispace1),
                        parser::parse_labeled,
                    ),
                    |(alias, expr)| {
                        Command::AddAlias(alias.to_owned(), VersionedRollExpr::V2(expr))
                    },
                ),
            ),
            preceded(
                pair(alt((tag_no_case("remove"), tag_no_case("r"))), multispace0),
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

fn parse_roll_info(input: &str) -> IResult<&str, Command> {
    preceded(
        pair(
            alt((
                tag_no_case("roll-info"),
                tag_no_case("roll_info"),
                tag_no_case("roll info"),
                tag_no_case("ri"),
            )),
            multispace0,
        ),
        alt((
            map(alt((tag_no_case("get"), tag_no_case("g"))), |_| {
                Command::GetRollInfo
            }),
            map(
                preceded(
                    pair(alt((tag_no_case("set"), tag_no_case("s"))), multispace0),
                    alt((
                        map(
                            alt((tag_no_case("true"), tag_no_case("t"), tag("1"))),
                            |_| true,
                        ),
                        map(
                            alt((tag_no_case("false"), tag_no_case("f"), tag_no_case("0"))),
                            |_| false,
                        ),
                    )),
                ),
                Command::SetRollInfo,
            ),
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
                    parse_roll_info,
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
            parser::parse_labeled,
            pair(multispace0, eof),
        ),
        |e| Command::Roll(VersionedRollExpr::V2(e)),
    )(input)
}

fn parse_extra_aliases(input: &str) -> IResult<&str, Vec<String>> {
    many0(map(
        preceded(
            many0(satisfy(|c| c != '$')),
            preceded(tag("$"), recognize(many1(chars_set))),
        ),
        |s| s.to_string(),
    ))(input)
}

pub async fn parse<Id: ClientId>(
    string: &str,
    id: Id,
    store: &StorageHandle<Id>,
) -> Option<(Command, String, bool)> {
    let storage_lookup = store
        .get(id.clone(), {
            let mut parsed = parse_extra_aliases(string)
                .map(|a| a.1)
                .unwrap_or_else(|_| Vec::new());
            parsed.push(string.to_string());
            parsed
        })
        .await;
    let prefix = storage_lookup.0;
    let roll_info = storage_lookup.3;
    if let Ok((_, c)) = parse_command(string, &prefix) {
        Some(c)
    } else if let Some(command) = storage_lookup
        .1
        .iter()
        .map(|prefix| parse_roll(string, prefix))
        .find_map(|r| r.ok().map(|res| res.1))
    {
        Some(command)
    } else if !storage_lookup.2.is_empty() {
        Some(Command::AliasRoll(storage_lookup.2))
    } else {
        None
    }
    .map(|c| (c, prefix, roll_info))
}

pub async fn parse_logging<Id: ClientId>(
    string: &str,
    id: Id,
    store: &StorageHandle<Id>,
) -> Option<(Command, String, bool)> {
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
            Ok((
                "",
                Command::Roll(VersionedRollExpr::V2(LabeledExpression::Unlabeled(
                    Expression::Simple(Term::Constant(1))
                )))
            ))
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
            parse_command("! r 1d4#label", "!"),
            Ok((
                "",
                Command::Roll(VersionedRollExpr::V2(LabeledExpression::Labeled(
                    Expression::Simple(Term::DiceThrow(SelectedDice::Unchanged(
                        FilteredDice::Simple(Dice {
                            throws: 1,
                            dice: DiceType::Number(4)
                        })
                    ))),
                    "label".to_string()
                )))
            ))
        );
    }

    #[test]
    fn test_chars_set() {
        assert_eq!(chars_set("ä"), Ok(("", 'ä')));
        assert_eq!(chars_set(":"), Ok(("", ':')));
        assert_eq!(chars_set("%"), Ok(("", '%')));
        assert_eq!(chars_set("✅"), Ok(("", '✅')));
    }
}
