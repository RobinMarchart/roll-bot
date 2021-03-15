use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use rand_xoshiro::Xoshiro256PlusPlus;
use robins_dice_roll::{dice_roll::DiceEvaluate, limits::DiceLimits};
use std::convert::{TryFrom, TryInto};

fn main() {
    let (dice, num) = {
        let mut args = std::env::args().skip(1);
        let dice = args
            .next()
            .and_then(|a| {
                robins_dice_roll::parser::parse_selected_dice(&a)
                    .ok()
                    .map(|r| r.1)
            })
            .expect("first arg should be selected dice");
        (
            dice,
            args.next()
                .and_then(|a| u32::from_str_radix(&a, 10).ok())
                .unwrap_or(1),
        )
    };
    let mut master_rng = ChaCha20Rng::from_entropy();

    let (result_min, result_max) = (dice.min(), dice.max());

    let mut results: Vec<i64> = vec![0; (result_max - result_min + 2).try_into().unwrap()];
    *results.get_mut(0).unwrap() = result_min;

    let (throw_min, throw_max) = {
        let dice_type = match match dice {
            robins_dice_roll::SelectedDice::Unchanged(d) => d,
            robins_dice_roll::SelectedDice::Selected(d, _, _) => d,
        } {
            robins_dice_roll::FilteredDice::Simple(d) => d,
            robins_dice_roll::FilteredDice::Filtered(d, _, _) => d,
        }
        .dice;
        (dice_type.min(), dice_type.max())
    };
    let mut throws: Vec<i64> = vec![0; (throw_max - throw_min + 2).try_into().unwrap()];
    *throws.get_mut(0).unwrap() = throw_min;

    for result in (0..num)
        .map(|_| {
            let mut seed: <Xoshiro256PlusPlus as SeedableRng>::Seed = Default::default();
            master_rng.fill(&mut seed);
            Xoshiro256PlusPlus::from_seed(seed)
        })
        .map(|mut r| dice.evaluate(&mut || false, &mut r))
    {
        let result = result.unwrap();
        let result_into = results
            .get_mut(
                usize::try_from(
                    result
                        .0
                        .iter()
                        .map(|i| i.clone())
                        .reduce(|i1, i2| i1 + i2)
                        .unwrap_or(1)
                        - result_min
                        + 1,
                )
                .unwrap(),
            )
            .unwrap();
        *result_into = *result_into + 1;

        result.1.iter().for_each(|r| {
            let throw_into = throws
                .get_mut(usize::try_from(r - throw_min + 1).unwrap())
                .unwrap();
            *throw_into = *throw_into + 1;
        })
    }

    npy::to_file("throws.npy", throws).unwrap();
    npy::to_file("rolls.npy", results).unwrap();
}
