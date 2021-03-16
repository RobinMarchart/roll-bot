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

use crate::dice_types::*;
use rand::{distributions::Uniform, Rng};
use std::convert::TryInto;

#[cfg(feature = "logging")]
use log::debug;

#[derive(Debug, PartialEq, Eq)]
pub enum EvaluationErrors {
    DivideByZero,
    Timeout,
    Overflow,
}

pub trait DiceEvaluate {
    fn evaluate<T: FnMut() -> bool, R: Rng>(
        &self,
        timeout_f: &mut T,
        rng: &mut R,
    ) -> Result<(Vec<i64>, Vec<i64>), EvaluationErrors>;
}

impl DiceEvaluate for Dice {
    fn evaluate<T: FnMut() -> bool, R: Rng>(
        &self,
        timeout_f: &mut T,
        rng: &mut R,
    ) -> Result<(Vec<i64>, Vec<i64>), EvaluationErrors> {
        if timeout_f() {
            return Err(EvaluationErrors::Timeout);
        }
        let mut rolls: Vec<i64> = Vec::with_capacity(self.throws.try_into().unwrap());
        let mut roll_counter: u8 = 0;
        match self.dice {
            DiceType::Number(faces) => {
                let dist = Uniform::new_inclusive(1, faces as i64);
                for _ in 0..self.throws {
                    roll_counter = roll_counter.wrapping_add(1);
                    if roll_counter == 0 && timeout_f() {
                        return Err(EvaluationErrors::Timeout);
                    }
                    rolls.push(rng.sample::<i64, _>(dist));
                }
            }
            DiceType::Fudge => {
                let dist: Uniform<i64> = Uniform::new_inclusive(-1, 1);
                for _ in 0..self.throws {
                    roll_counter = roll_counter.wrapping_add(1);
                    if roll_counter == 0 && timeout_f() {
                        return Err(EvaluationErrors::Timeout);
                    }
                    rolls.push(rng.sample(dist));
                }
            }
            DiceType::Multiply(base_faces) => {
                let dist = Uniform::new_inclusive(1, base_faces as i64);
                for _ in 0..self.throws {
                    roll_counter = roll_counter.wrapping_add(1);
                    if roll_counter == 0 && timeout_f() {
                        return Err(EvaluationErrors::Timeout);
                    }
                    rolls.push(
                        rng.sample(dist)
                            .checked_mul(rng.sample(dist))
                            .ok_or(EvaluationErrors::Overflow)?,
                    );
                }
            }
        }

        #[cfg(feature = "logging")]
        {
            debug!("Dice roll result for {} is {:?}", &self, &rolls);
        }

        let rolls_copy = rolls.clone();
        Ok((rolls, rolls_copy))
    }
}

impl DiceEvaluate for FilteredDice {
    fn evaluate<T: FnMut() -> bool, R: Rng>(
        &self,
        timeout_f: &mut T,
        rng: &mut R,
    ) -> Result<(Vec<i64>, Vec<i64>), EvaluationErrors> {
        let result = match self {
            FilteredDice::Simple(dice) => dice.evaluate(timeout_f, rng),
            FilteredDice::Filtered(dice, filter, target) => {
                dice.evaluate(timeout_f, rng).map(|original| {
                    (
                        original
                            .0
                            .into_iter()
                            .filter(match filter {
                                Filter::Bigger => {
                                    Box::new(|i: &i64| i > &(target.to_owned() as i64))
                                        as Box<dyn Fn(&i64) -> bool>
                                }
                                Filter::BiggerEq => {
                                    Box::new(|i: &i64| i > &(target.to_owned() as i64))
                                        as Box<dyn Fn(&i64) -> bool>
                                }
                                Filter::Smaller => {
                                    Box::new(|i: &i64| i < &(target.to_owned() as i64))
                                        as Box<dyn Fn(&i64) -> bool>
                                }
                                Filter::SmallerEq => {
                                    Box::new(|i: &i64| i <= &(target.to_owned() as i64))
                                        as Box<dyn Fn(&i64) -> bool>
                                }
                                Filter::NotEq => {
                                    Box::new(|i: &i64| i != &(target.to_owned() as i64))
                                        as Box<dyn Fn(&i64) -> bool>
                                }
                            })
                            .collect(),
                        original.1,
                    )
                })
            }
        };
        #[cfg(feature = "logging")]
        {
            debug!("rolled {:?} for filtered dice {}", &result, &self)
        }
        result
    }
}

impl DiceEvaluate for SelectedDice {
    fn evaluate<T: FnMut() -> bool, R: Rng>(
        &self,
        timeout_f: &mut T,
        rng: &mut R,
    ) -> Result<(Vec<i64>, Vec<i64>), EvaluationErrors> {
        let result = match self {
            SelectedDice::Unchanged(dice) => dice.evaluate(timeout_f, rng),
            SelectedDice::Selected(dice, selector, max_size) => {
                dice.evaluate(timeout_f, rng)
                    .map(|original: (Vec<i64>, Vec<i64>)| {
                        if original.0.len() > max_size.to_owned() as usize {
                            let range = match selector {
                                Selector::Higher => {
                                    (original.0.len() - max_size.to_owned() as usize)
                                        ..original.0.len()
                                }
                                Selector::Lower => (0..(max_size.to_owned() as usize)),
                            };
                            let mut source = original;
                            source.0.sort_unstable();
                            (source.0[range].to_vec(), source.1)
                        } else {
                            original
                        }
                    })
            }
        };
        #[cfg(feature = "logging")]
        {
            debug!("rolled {:?} for selected dice {}", &result, &self)
        }
        result
    }
}

pub trait TermEvaluate {
    fn evaluate<T: FnMut() -> bool, R: Rng>(
        &self,
        timeout_f: &mut T,
        rng: &mut R,
    ) -> Result<(i64, Vec<i64>), EvaluationErrors>;
}

impl TermEvaluate for Term {
    fn evaluate<T: FnMut() -> bool, R: Rng>(
        &self,
        timeout_f: &mut T,
        rng: &mut R,
    ) -> Result<(i64, Vec<i64>), EvaluationErrors> {
        let result = match self {
            Term::Constant(i) => Ok((i.to_owned(), Vec::new())),
            Term::DiceThrow(dice) => dice.evaluate(timeout_f, rng).map(|roll_results| {
                (
                    roll_results.0.into_iter().reduce(|a, b| a + b).unwrap_or(0),
                    roll_results.1,
                )
            }),
            Term::SubTerm(term) => term.evaluate(timeout_f, rng),
            Term::Calculation(left, op, right) => {
                let left_r = left.evaluate(timeout_f, rng)?;
                let right_r = right.evaluate(timeout_f, rng)?;
                let result = match op {
                    Operation::Add => left_r
                        .0
                        .checked_add(right_r.0)
                        .ok_or(EvaluationErrors::Overflow),
                    Operation::Sub => left_r
                        .0
                        .checked_sub(right_r.0)
                        .ok_or(EvaluationErrors::Overflow),
                    Operation::Mul => left_r
                        .0
                        .checked_mul(right_r.0)
                        .ok_or(EvaluationErrors::Overflow),
                    Operation::Div => left_r
                        .0
                        .checked_div(right_r.0)
                        .ok_or(EvaluationErrors::DivideByZero),
                }?;
                Ok((result, [left_r.1, right_r.1].concat()))
            }
        };
        #[cfg(feature = "logging")]
        {
            debug!("got {:?} for term {}", &result, &self)
        }
        result
    }
}

impl TermEvaluate for Box<Term> {
    fn evaluate<T: FnMut() -> bool, R: Rng>(
        &self,
        timeout_f: &mut T,
        rng: &mut R,
    ) -> Result<(i64, Vec<i64>), EvaluationErrors> {
        self.as_ref().evaluate(timeout_f, rng)
    }
}

pub trait ExpressionEvaluate {
    fn evaluate<T: FnMut() -> bool, R: Rng>(
        &self,
        timeout_t: &mut T,
        rng: &mut R,
    ) -> Result<Vec<(i64, Vec<i64>)>, EvaluationErrors>;
}

impl ExpressionEvaluate for Expression {
    fn evaluate<T: FnMut() -> bool, R: Rng>(
        &self,
        timeout_f: &mut T,
        rng: &mut R,
    ) -> Result<Vec<(i64, Vec<i64>)>, EvaluationErrors> {
        match self {
            Expression::Simple(term) => term.evaluate(timeout_f, rng).map(|res| vec![res]),
            Expression::List(count, term) => {
                let size: usize = (*count).try_into().expect("failed to convert u32 to usize");
                let mut result_collector: Vec<(i64, Vec<i64>)> = Vec::with_capacity(size);
                for _ in 0..size {
                    result_collector.push(term.evaluate(timeout_f, rng)?);
                }
                Ok(result_collector)
            }
        }
    }
}
