pub trait DiceLimits {
    fn min(&self) -> i64;
    fn max(&self) -> i64;
}

use crate::dice_types::*;

impl DiceLimits for DiceType {
    fn min(&self) -> i64 {
        match self {
            DiceType::Number(_) => 1,
            DiceType::Fudge => -1,
            DiceType::Multiply(_) => 1,
        }
    }

    fn max(&self) -> i64 {
        match self {
            DiceType::Number(n) => (*n).into(),
            DiceType::Fudge => 1,
            DiceType::Multiply(n) => i64::from(*n) * i64::from(*n),
        }
    }
}

impl DiceLimits for Dice {
    fn min(&self) -> i64 {
        i64::from(self.throws) * self.dice.min()
    }

    fn max(&self) -> i64 {
        i64::from(self.throws) * self.dice.max()
    }
}
impl DiceLimits for FilteredDice {
    fn min(&self) -> i64 {
        match self {
            FilteredDice::Simple(d) => d.min(),
            FilteredDice::Filtered(d, _, _) => d.min(),
        }
    }

    fn max(&self) -> i64 {
        match self {
            FilteredDice::Simple(d) => d.max(),
            FilteredDice::Filtered(d, _, _) => d.max(),
        }
    }
}
impl DiceLimits for SelectedDice {
    fn min(&self) -> i64 {
        match self {
            SelectedDice::Unchanged(d) => d.min(),
            SelectedDice::Selected(d, _, n) => {
                match d {
                    FilteredDice::Simple(dc) => dc,
                    FilteredDice::Filtered(dc, _, _) => dc,
                }
                .dice
                .min()
                    * i64::from(*n)
            }
        }
    }

    fn max(&self) -> i64 {
        match self {
            SelectedDice::Unchanged(d) => d.max(),
            SelectedDice::Selected(d, _, n) => {
                match d {
                    FilteredDice::Simple(dc) => dc,
                    FilteredDice::Filtered(dc, _, _) => dc,
                }
                .dice
                .max()
                    * i64::from(*n)
            }
        }
    }
}
