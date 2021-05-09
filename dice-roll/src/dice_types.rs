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

use std::fmt::{self, Debug};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum DiceType {
    Number(u32),
    Fudge,
    Multiply(u32),
}

impl fmt::Display for DiceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiceType::Number(n) => {
                write!(f, "d{}", n)
            }
            DiceType::Fudge => {
                write!(f, "dF")
            }
            DiceType::Multiply(n) => {
                write!(f, "d{}x", n)
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Dice {
    pub throws: u32,
    pub dice: DiceType,
}

impl fmt::Display for Dice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.throws, self.dice)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Filter {
    Bigger,
    BiggerEq,
    Smaller,
    SmallerEq,
    NotEq,
}

impl fmt::Display for Filter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Filter::Bigger => {
                write!(f, ">")
            }
            Filter::BiggerEq => {
                write!(f, ">=")
            }
            Filter::Smaller => {
                write!(f, "<")
            }
            Filter::SmallerEq => {
                write!(f, "<=")
            }
            Filter::NotEq => {
                write!(f, "!=")
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum FilteredDice {
    Simple(Dice),
    Filtered(Dice, Filter, u32),
}

impl fmt::Display for FilteredDice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FilteredDice::Simple(d) => {
                write!(f, "{}", d)
            }
            FilteredDice::Filtered(d, fil, n) => {
                write!(f, "{}{}{}", d, fil, n)
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Selector {
    Higher,
    Lower,
}

impl fmt::Display for Selector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Selector::Higher => {
                write!(f, "h")
            }
            Selector::Lower => {
                write!(f, "l")
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum SelectedDice {
    Unchanged(FilteredDice),
    Selected(FilteredDice, Selector, u32),
}

impl fmt::Display for SelectedDice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SelectedDice::Unchanged(d) => {
                write!(f, "{}", d)
            }
            SelectedDice::Selected(d, s, n) => {
                write!(f, "{}{}{}", d, s, n)
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Operation {
    Mul,
    Div,
    Add,
    Sub,
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operation::Mul => {
                write!(f, "*")
            }
            Operation::Div => {
                write!(f, "/")
            }
            Operation::Add => {
                write!(f, "+")
            }
            Operation::Sub => {
                write!(f, "-")
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Term {
    Constant(i64),
    DiceThrow(SelectedDice),
    Calculation(Box<Term>, Operation, Box<Term>),
    SubTerm(Box<Term>),
}

impl fmt::Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Term::Constant(c) => {
                write!(f, "{}", c)
            }
            Term::DiceThrow(d) => {
                write!(f, "{}", d)
            }
            Term::Calculation(l, op, r) => {
                write!(f, "{} {} {}", l, op, r)
            }
            Term::SubTerm(t) => {
                write!(f, "({})", t)
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Expression {
    Simple(Term),
    List(u32, Term),
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expression::Simple(t) => {
                write!(f, "{}", t)
            }
            Expression::List(n, t) => {
                write!(f, "{}{{{}}}", n, t)
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum LabeledExpression {
    Unlabeled(Expression),
    Labeled(Expression, String),
}

impl fmt::Display for LabeledExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LabeledExpression::Unlabeled(e) => {
                write!(f, "{}", e)
            }
            LabeledExpression::Labeled(e, _) => {
                write!(f, "{}", e)
            }
        }
    }
}
