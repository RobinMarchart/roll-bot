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

#[cfg(feature = "serde-support")]
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub enum DiceType {
    Number(u32),
    Fudge,
    Multiply(u32),
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct Dice {
    pub throws: u32,
    pub dice: DiceType,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub enum Filter {
    Bigger,
    BiggerEq,
    Smaller,
    SmallerEq,
    NotEq,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub enum FilteredDice {
    Simple(Dice),
    Filtered(Dice, Filter, u32),
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub enum Selector {
    Higher,
    Lower,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub enum SelectedDice {
    Unchanged(FilteredDice),
    Selected(FilteredDice, Selector, u32),
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub enum Operation {
    Mul,
    Div,
    Add,
    Sub,
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub enum Term {
    Constant(i64),
    DiceThrow(SelectedDice),
    Calculation(Box<Term>, Operation, Box<Term>),
    SubTerm(Box<Term>),
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub enum Expression {
    Simple(Term),
    List(u32, Term),
}
