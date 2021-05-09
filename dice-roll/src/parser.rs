/*
Copyright 2021 Robin Marchart

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
*/

use crate::{
    dice_types::{
        Dice, DiceType, Expression, Filter, FilteredDice, Operation, SelectedDice, Selector, Term,
    },
    LabeledExpression,
};

use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case},
    character::complete::{digit1, multispace0, satisfy},
    combinator::{map, map_res, opt, recognize, success, verify},
    error::context,
    multi::{many0, many1},
    sequence::{delimited, pair, preceded, terminated, tuple},
    IResult,
};

pub fn parse_dice_digit(input: &str) -> IResult<&str, &str> {
    alt((tag_no_case("d"), tag_no_case("w")))(input)
}

pub fn parse_dice_type(input: &str) -> IResult<&str, DiceType> {
    alt((
        map(
            terminated(parse_u32, terminated(multispace0, tag_no_case("x"))),
            DiceType::Multiply,
        ),
        map(parse_u32, DiceType::Number),
        map(tag_no_case("f"), |_| DiceType::Fudge),
        map(tag("%"), |_| DiceType::Number(100)),
    ))(input)
}

pub fn parse_u32(input: &str) -> IResult<&str, u32> {
    context(
        "Failed to parse integer between 1 and 4294967295 inclusive",
        verify(
            map_res(digit1, |s: &str| s.parse::<u32>()),
            |value: &u32| value > &0,
        ),
    )(input)
}

pub fn parse_i64(input: &str) -> IResult<&str, i64> {
    map_res(
        recognize(pair(alt((tag("+"), tag("-"), success(""))), digit1)),
        |s: &str| s.parse::<i64>(),
    )(input)
}

pub fn parse_dice(input: &str) -> IResult<&str, Dice> {
    map(
        tuple((
            terminated(alt((parse_u32, success(1))), multispace0),
            preceded(parse_dice_digit, preceded(multispace0, parse_dice_type)),
        )),
        |dice_params| Dice {
            throws: dice_params.0,
            dice: dice_params.1,
        },
    )(input)
}

pub fn parse_filter(input: &str) -> IResult<&str, Filter> {
    alt((
        map(tag(">="), |_| Filter::BiggerEq),
        map(tag(">"), |_| Filter::Bigger),
        map(tag("<="), |_| Filter::SmallerEq),
        map(tag("<"), |_| Filter::Smaller),
        map(tag("!="), |_| Filter::NotEq),
    ))(input)
}

pub fn parse_filtered_dice(input: &str) -> IResult<&str, FilteredDice> {
    alt((
        map(
            tuple((
                parse_dice,
                delimited(multispace0, parse_filter, multispace0),
                parse_u32,
            )),
            |res| FilteredDice::Filtered(res.0, res.1, res.2),
        ),
        map(parse_dice, FilteredDice::Simple),
    ))(input)
}

pub fn parse_selector(input: &str) -> IResult<&str, Selector> {
    alt((
        map(alt((tag_no_case("h"), tag_no_case("k"))), |_| {
            Selector::Higher
        }),
        map(tag_no_case("l"), |_| Selector::Lower),
    ))(input)
}

pub fn parse_selected_dice(input: &str) -> IResult<&str, SelectedDice> {
    alt((
        map(
            tuple((
                parse_filtered_dice,
                delimited(multispace0, parse_selector, multispace0),
                parse_u32,
            )),
            |select| SelectedDice::Selected(select.0, select.1, select.2),
        ),
        map(parse_filtered_dice, SelectedDice::Unchanged),
    ))(input)
}

pub fn parse_term(input: &str) -> IResult<&str, Term> {
    alt((
        parse_term_calculation,
        parse_term_roll,
        parse_term_constant,
        parse_term_subterm,
    ))(input)
}

pub fn parse_term_constant(input: &str) -> IResult<&str, Term> {
    map(parse_i64, Term::Constant)(input)
}

pub fn parse_term_subterm(input: &str) -> IResult<&str, Term> {
    map(
        delimited(
            tag("("),
            delimited(multispace0, parse_term, multispace0),
            tag(")"),
        ),
        |subterm| Term::SubTerm(Box::new(subterm)),
    )(input)
}

pub fn parse_term_roll(input: &str) -> IResult<&str, Term> {
    map(parse_selected_dice, Term::DiceThrow)(input)
}

pub fn parse_operator(input: &str) -> IResult<&str, Operation> {
    alt((
        map(tag("+"), |_| Operation::Add),
        map(tag("-"), |_| Operation::Sub),
        map(tag("*"), |_| Operation::Mul),
        map(tag("/"), |_| Operation::Div),
    ))(input)
}

pub fn parse_term_calculation(input: &str) -> IResult<&str, Term> {
    map(
        tuple((
            alt((parse_term_roll, parse_term_constant, parse_term_subterm)),
            delimited(multispace0, parse_operator, multispace0),
            parse_term,
        )),
        |calc| Term::Calculation(Box::new(calc.0), calc.1, Box::new(calc.2)),
    )(input)
}

fn rearange_term(root: Term) -> Term {
    if let Term::Calculation(left_top, op_top, right_top) = root {
        if op_top == Operation::Mul || op_top == Operation::Div {
            if let Term::Calculation(left_child, op_child, right_child) = *right_top {
                Term::Calculation(
                    Box::new(Term::Calculation(left_top, op_top, left_child)),
                    op_child,
                    Box::new(rearange_term(*right_child)),
                )
            } else {
                Term::Calculation(left_top, op_top, Box::new(rearange_term(*right_top)))
            }
        } else {
            Term::Calculation(left_top, op_top, Box::new(rearange_term(*right_top)))
        }
    } else if let Term::SubTerm(term) = root {
        Term::SubTerm(Box::new(rearange_term(*term)))
    } else {
        root
    }
}

pub fn parse_rearanged_term(input: &str) -> IResult<&str, Term> {
    map(parse_term, rearange_term)(input)
}

pub fn parse_expression(input: &str) -> IResult<&str, Expression> {
    alt((
        map(
            pair(
                parse_u32,
                preceded(
                    multispace0,
                    delimited(
                        tag("{"),
                        delimited(multispace0, parse_rearanged_term, multispace0),
                        tag("}"),
                    ),
                ),
            ),
            |list| Expression::List(list.0, list.1),
        ),
        map(parse_rearanged_term, Expression::Simple),
    ))(input)
}

pub fn parse_labeled(input: &str) -> IResult<&str, LabeledExpression> {
    map(
        pair(
            parse_expression,
            opt(preceded(
                pair(tag("#"), multispace0),
                map(
                    many0(terminated(
                        recognize(many1(satisfy(|c| !(c.is_whitespace() || c == '\n')))),
                        multispace0,
                    )),
                    |labels: Vec<&str>| {
                        labels
                            .iter()
                            .map(|s| s.to_string())
                            .reduce(|l1, l2| format!("{} {}", l1, l2))
                            .unwrap_or_else(|| "".to_string())
                    },
                ),
            )),
        ),
        |r| match r {
            (e, Some(l)) => LabeledExpression::Labeled(e, l),
            (e, None) => LabeledExpression::Unlabeled(e),
        },
    )(input)
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_parse_dice_digit() {
        assert_eq!(parse_dice_digit("d"), Ok(("", "d")));
        assert_eq!(parse_dice_digit("D"), Ok(("", "D")));
        assert_eq!(parse_dice_digit("w"), Ok(("", "w")));
        assert_eq!(parse_dice_digit("W"), Ok(("", "W")));
        assert_eq!(parse_dice_digit("dd"), Ok(("d", "d")));
        assert_eq!(parse_dice_digit("d%"), Ok(("%", "d")));
        assert!(parse_dice_digit("l").is_err());
        assert!(parse_dice_digit("%").is_err());
        assert!(parse_dice_digit("").is_err());
    }

    #[test]
    fn test_parse_u32() {
        assert_eq!(parse_u32("1"), Ok(("", 1)));
        assert_eq!(parse_u32("6969"), Ok(("", 6969)));
        assert_eq!(parse_u32("4294967295"), Ok(("", 4294967295)));
        assert!(parse_u32("4294967296").is_err());
        assert!(parse_u32("-1").is_err());
        assert!(parse_u32("").is_err());
        assert!(parse_u32("0").is_err());
    }

    #[test]
    fn test_parse_i64() {
        assert_eq!(parse_i64("0"), Ok(("", 0)));
        assert_eq!(parse_i64("1"), Ok(("", 1)));
        assert_eq!(parse_i64("+1"), Ok(("", 1)));
        assert_eq!(parse_i64("-1"), Ok(("", -1)));
        assert_eq!(parse_i64("6969"), Ok(("", 6969)));
        assert_eq!(parse_i64("+6969"), Ok(("", 6969)));
        assert_eq!(parse_i64("-1337"), Ok(("", -1337)));
        assert_eq!(
            parse_i64("-9223372036854775808"),
            Ok(("", -9223372036854775808))
        );
        assert_eq!(
            parse_i64("9223372036854775807"),
            Ok(("", 9223372036854775807))
        );
        assert_eq!(
            parse_i64("+9223372036854775807"),
            Ok(("", 9223372036854775807))
        );
        assert_eq!(parse_i64("0k"), Ok(("k", 0)));
        assert!(parse_i64("k").is_err());
        assert!(parse_i64("").is_err());
    }

    #[test]
    fn test_parse_dice_type() {
        assert_eq!(parse_dice_type("1"), Ok(("", DiceType::Number(1))));
        assert_eq!(parse_dice_type("1337"), Ok(("", DiceType::Number(1337))));
        assert_eq!(parse_dice_type("%"), Ok(("", DiceType::Number(100))));
        assert_eq!(parse_dice_type("f"), Ok(("", DiceType::Fudge)));
        assert_eq!(parse_dice_type("F"), Ok(("", DiceType::Fudge)));
        assert_eq!(parse_dice_type("1x"), Ok(("", DiceType::Multiply(1))));
        assert_eq!(parse_dice_type("6969X"), Ok(("", DiceType::Multiply(6969))));
        assert_eq!(
            parse_dice_type("1337 x"),
            Ok(("", DiceType::Multiply(1337)))
        );
        assert!(parse_dice_type("x").is_err());
        assert!(parse_dice_type("").is_err());
    }

    #[test]
    fn test_parse_dice() {
        assert_eq!(
            parse_dice("d1"),
            Ok((
                "",
                Dice {
                    throws: 1,
                    dice: DiceType::Number(1)
                }
            ))
        );
        assert_eq!(
            parse_dice("1D %"),
            Ok((
                "",
                Dice {
                    throws: 1,
                    dice: DiceType::Number(100)
                }
            ))
        );
        assert_eq!(
            parse_dice("20w  \t3\tX"),
            Ok((
                "",
                Dice {
                    throws: 20,
                    dice: DiceType::Multiply(3)
                }
            ))
        );
    }

    #[test]
    fn test_parse_filter() {
        assert_eq!(parse_filter("<"), Ok(("", Filter::Smaller)));
        assert_eq!(parse_filter("<="), Ok(("", Filter::SmallerEq)));
        assert_eq!(parse_filter(">"), Ok(("", Filter::Bigger)));
        assert_eq!(parse_filter(">="), Ok(("", Filter::BiggerEq)));
        assert_eq!(parse_filter("!="), Ok(("", Filter::NotEq)));
        assert_eq!(parse_filter("!=3"), Ok(("3", Filter::NotEq)));
        assert!(parse_filter("==").is_err());
        assert!(parse_filter("").is_err());
    }

    #[test]
    fn test_parse_filtered_dice() {
        assert_eq!(
            parse_filtered_dice("d4"),
            Ok((
                "",
                FilteredDice::Simple(Dice {
                    throws: 1,
                    dice: DiceType::Number(4)
                })
            ))
        );
        assert_eq!(
            parse_filtered_dice("2d2!=2"),
            Ok((
                "",
                FilteredDice::Filtered(
                    Dice {
                        throws: 2,
                        dice: DiceType::Number(2)
                    },
                    Filter::NotEq,
                    2
                )
            ))
        );
        assert_eq!(
            parse_filtered_dice("10   w  10  \t x \t  < \t 75"),
            Ok((
                "",
                FilteredDice::Filtered(
                    Dice {
                        throws: 10,
                        dice: DiceType::Multiply(10)
                    },
                    Filter::Smaller,
                    75
                )
            ))
        );
        assert_eq!(
            parse_filtered_dice("69d69>"),
            Ok((
                ">",
                FilteredDice::Simple(Dice {
                    throws: 69,
                    dice: DiceType::Number(69)
                })
            ))
        );
        assert!(parse_filtered_dice("").is_err());
    }

    #[test]
    fn test_parse_selector() {
        assert_eq!(parse_selector("h"), Ok(("", Selector::Higher)));
        assert_eq!(parse_selector("H"), Ok(("", Selector::Higher)));
        assert_eq!(parse_selector("k"), Ok(("", Selector::Higher)));
        assert_eq!(parse_selector("K"), Ok(("", Selector::Higher)));
        assert_eq!(parse_selector("l"), Ok(("", Selector::Lower)));
        assert_eq!(parse_selector("L"), Ok(("", Selector::Lower)));
        assert_eq!(parse_selector("hl"), Ok(("l", Selector::Higher)));
        assert!(parse_selector("").is_err());
    }

    #[test]
    fn test_parse_selected_dice() {
        assert_eq!(
            parse_selected_dice("d3"),
            Ok((
                "",
                SelectedDice::Unchanged(FilteredDice::Simple(Dice {
                    throws: 1,
                    dice: DiceType::Number(3)
                }))
            ))
        );
        assert_eq!(
            parse_selected_dice("4W10X>50k2"),
            Ok((
                "",
                SelectedDice::Selected(
                    FilteredDice::Filtered(
                        Dice {
                            throws: 4,
                            dice: DiceType::Multiply(10)
                        },
                        Filter::Bigger,
                        50
                    ),
                    Selector::Higher,
                    2
                )
            ))
        );
        assert_eq!(
            parse_selected_dice("4\t  W \t 10  \tX\t  >\t  50\t  k \t 2"),
            Ok((
                "",
                SelectedDice::Selected(
                    FilteredDice::Filtered(
                        Dice {
                            throws: 4,
                            dice: DiceType::Multiply(10)
                        },
                        Filter::Bigger,
                        50
                    ),
                    Selector::Higher,
                    2
                )
            ))
        );
        assert!(parse_selected_dice("").is_err());
    }

    #[test]
    fn test_parse_term() {
        assert!(parse_term("d 3 + d f + d % + 1337 d 69 x * 4 d 100 / ( 3 w 10 - 2 )").is_ok());
        assert_eq!(
            parse_term("d 3 + 66DF * 4d3x - 1"),
            Ok((
                "",
                Term::Calculation(
                    Box::new(Term::DiceThrow(SelectedDice::Unchanged(
                        FilteredDice::Simple(Dice {
                            throws: 1,
                            dice: DiceType::Number(3)
                        })
                    ))),
                    Operation::Add,
                    Box::new(Term::Calculation(
                        Box::new(Term::DiceThrow(SelectedDice::Unchanged(
                            FilteredDice::Simple(Dice {
                                throws: 66,
                                dice: DiceType::Fudge
                            })
                        ))),
                        Operation::Mul,
                        Box::new(Term::Calculation(
                            Box::new(Term::DiceThrow(SelectedDice::Unchanged(
                                FilteredDice::Simple(Dice {
                                    throws: 4,
                                    dice: DiceType::Multiply(3)
                                })
                            ))),
                            Operation::Sub,
                            Box::new(Term::Constant(1))
                        ))
                    ))
                )
            ))
        );
        assert!(parse_term("").is_err())
    }

    fn test_parse_expr() {}
}
