#![deny(
    bad_style,
    const_err,
    dead_code,
    improper_ctypes,
    missing_debug_implementations,
    no_mangle_generic_items,
    non_shorthand_field_patterns,
    overflowing_literals,
    path_statements,
    patterns_in_fns_without_body,
    private_in_public,
    unconditional_recursion,
    unreachable_pub,
    unused,
    unused_allocation,
    unused_comparisons,
    unused_parens,
    while_true,
    clippy::expect_used
)]
#![deny(unsafe_code)]

use crate::words::{EXTENDED_WORDS, TARGET_WORDS};
use crate::LetterGuess::NotUsed;
use std::fmt::{Debug, Display, Formatter, Write};
use std::iter::Zip;
use thiserror::Error;

pub mod words;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Word([u8; 5]);

#[derive(Debug, Error)]
pub enum WordError {
    #[error("Words should be ASCII letters only.  Got '{0}' which contains '{1}'")]
    Chars(String, char),
    #[error("Words have five letters, got a string containing {0} bytes")]
    Length(usize),
    #[error("Words not in the word list: {0}")]
    NotWord(String),
    #[error("Input doesn't look like a Worlde share")]
    NotWordle,
    #[error("Unknown Lua Error")]
    Unknown,
}

impl TryFrom<&str> for Word {
    type Error = WordError;

    fn try_from(value: &str) -> Result<Word, WordError> {
        for x in value.chars() {
            if !('a'..='z').contains(&x) {
                return Err(WordError::Chars(value.into(), x));
            }
        }
        let b = value.as_bytes();
        if b.len() != 5 {
            return Err(WordError::Length(b.len()));
        }
        let mut r: [u8; 5] = [0; 5];
        r.copy_from_slice(b);
        let w = Word(r);
        if !TARGET_WORDS.contains(&w) && !EXTENDED_WORDS.contains(&w) {
            return Err(WordError::NotWord(value.into()));
        }
        Ok(w)
    }
}

impl Display for Word {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for ch in self.0.iter() {
            let ch = char::from_u32(*ch as u32).unwrap();
            f.write_char(ch)?;
        }
        Ok(())
    }
}

impl Debug for Word {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Word({})", self))
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum LetterGuess {
    Correct,
    Misplaced,
    NotUsed,
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct GuessStatus(pub [LetterGuess; 5]);
// Wordle 232 6/6:black_large_square::large_yellow_square::large_green_square::black_large_square::black_large_square:
// :black_large_square::black_large_square::black_large_square::black_large_square::large_yellow_square:
// :large_green_square::large_yellow_square::large_green_square::black_large_square::black_large_square:
// :large_green_square::black_large_square::large_green_square::large_green_square::large_green_square:
// :black_large_square::large_yellow_square::black_large_square::large_yellow_square::large_yellow_square:
// :large_green_square::large_green_square::large_green_square::large_green_square::large_green_square:
impl TryFrom<&str> for GuessStatus {
    type Error = WordError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let value = if value.contains(':') {
            value
                .replacen(":black_large_square:", "-", 5)
                .replacen(":large_yellow_square:", "+", 5)
                .replacen(":large_green_square:", "=", 5)
        } else {
            value.to_string()
        };
        let chars: Vec<char> = value.chars().collect();
        for &x in chars.iter() {
            if !"=+-🟩🟨⬛".contains(x) {
                return Err(WordError::Chars(value, x));
            }
        }
        if chars.len() != 5 {
            return Err(WordError::Length(chars.len()));
        }
        let mut r: [LetterGuess; 5] = [NotUsed; 5];
        for (status, symbol) in r.iter_mut().zip(chars.into_iter()) {
            match symbol {
                '=' | '🟩' => *status = LetterGuess::Correct,
                '+' | '🟨' => *status = LetterGuess::Misplaced,
                '-' | '⬛' => *status = LetterGuess::NotUsed,
                x => return Err(WordError::Chars(value, x)),
            }
        }
        Ok(GuessStatus(r))
    }
}

impl Display for GuessStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for status in self.0 {
            f.write_char(match status {
                LetterGuess::Correct => '🟩',
                LetterGuess::Misplaced => '🟨',
                LetterGuess::NotUsed => '⬛',
            })?;
        }
        Ok(())
    }
}

impl Debug for GuessStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("GuessStatus({})", self))
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct WordGuess {
    word: Word,
    pub status: GuessStatus,
}

fn zip4<A, B, C, D>(a: A, b: B, c: C, d: D) -> Zip<Zip<A, B>, Zip<C, D>>
where
    A: Iterator,
    B: Iterator,
    C: Iterator,
    D: Iterator,
{
    Iterator::zip(Iterator::zip(a, b), Iterator::zip(c, d))
}

impl WordGuess {
    pub fn guess(guess: Word, target: Word) -> WordGuess {
        let mut available = target.0;
        let mut result: [LetterGuess; 5] = [LetterGuess::NotUsed; 5];
        for ((t, g), (a, r)) in zip4(
            target.0.iter(),
            guess.0.iter(),
            available.iter_mut(),
            result.iter_mut(),
        ) {
            if t == g {
                *a = 0;
                *r = LetterGuess::Correct;
            }
        }
        for (r, g) in result.iter_mut().zip(guess.0) {
            if *r != LetterGuess::Correct {
                for a in available.iter_mut() {
                    if *a == g {
                        *a = 0;
                        *r = LetterGuess::Misplaced;
                        break;
                    }
                }
            }
        }
        WordGuess {
            word: guess,
            status: GuessStatus(result),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{LetterGuess, Word, WordGuess, TARGET_WORDS};
    use anyhow::Error;

    #[test]
    fn guess_correct() -> Result<(), Error> {
        let guess = Word::try_from("cigar")?;
        let target = Word::try_from("cigar")?;
        let result = WordGuess::guess(guess, target);
        assert_eq!(result.status.0, [LetterGuess::Correct; 5]);
        Ok(())
    }

    #[test]
    fn guess_incorrect() -> Result<(), Error> {
        let guess = Word::try_from("humph")?;
        let target = Word::try_from("cigar")?;
        let result = WordGuess::guess(guess, target);
        assert_eq!(result.status.0, [LetterGuess::NotUsed; 5]);
        Ok(())
    }

    #[test]
    fn guess_one_correct() -> Result<(), Error> {
        let guess = Word::try_from("sissy")?;
        let target = Word::try_from("cigar")?;
        let result = WordGuess::guess(guess, target);
        let mut expected = [LetterGuess::NotUsed; 5];
        expected[1] = LetterGuess::Correct;
        assert_eq!(result.status.0, expected);
        Ok(())
    }

    #[test]
    fn guess_one_misplaced() -> Result<(), Error> {
        let guess = Word::try_from("heath")?;
        let target = Word::try_from("cigar")?;
        let result = WordGuess::guess(guess, target);
        let mut expected = [LetterGuess::NotUsed; 5];
        expected[2] = LetterGuess::Misplaced;
        assert_eq!(result.status.0, expected);
        Ok(())
    }

    #[test]
    fn guess_multi_guess_correct() -> Result<(), Error> {
        let guess = Word::try_from("skill")?;
        let target = Word::try_from("panel")?;
        let result = WordGuess::guess(guess, target);
        let mut expected = [LetterGuess::NotUsed; 5];
        expected[4] = LetterGuess::Correct;
        assert_eq!(result.status.0, expected);
        Ok(())
    }

    #[test]
    fn guess_multi_guess_misplaced() -> Result<(), Error> {
        let guess = Word::try_from("skill")?;
        let target = Word::try_from("labor")?;
        let result = WordGuess::guess(guess, target);
        let mut expected = [LetterGuess::NotUsed; 5];
        expected[3] = LetterGuess::Misplaced;
        assert_eq!(result.status.0, expected);
        Ok(())
    }

    #[test]
    fn guess_multi_target_correct() -> Result<(), Error> {
        let guess = Word::try_from("panel")?;
        let target = Word::try_from("skill")?;
        let result = WordGuess::guess(guess, target);
        let mut expected = [LetterGuess::NotUsed; 5];
        expected[4] = LetterGuess::Correct;
        assert_eq!(result.status.0, expected);
        Ok(())
    }

    #[test]
    fn guess_multi_target_misplaced() -> Result<(), Error> {
        let guess = Word::try_from("labor")?;
        let target = Word::try_from("skill")?;
        let result = WordGuess::guess(guess, target);
        let mut expected = [LetterGuess::NotUsed; 5];
        expected[0] = LetterGuess::Misplaced;
        assert_eq!(result.status.0, expected);
        Ok(())
    }

    #[test]
    fn guess_multi_correct_and_misplaced() -> Result<(), Error> {
        let guess = Word::try_from("label")?;
        let target = Word::try_from("skill")?;
        let result = WordGuess::guess(guess, target);
        let mut expected = [LetterGuess::NotUsed; 5];
        expected[0] = LetterGuess::Misplaced;
        expected[4] = LetterGuess::Correct;
        assert_eq!(result.status.0, expected);
        Ok(())
    }

    #[test]
    fn index_of_day_works() {
        assert_eq!(
            TARGET_WORDS
                .split(|w| *w == Word::try_from("those").unwrap())
                .next()
                .unwrap()
                .len(),
            227
        )
    }

    #[test]
    fn index_into_day_works() {
        assert_eq!(TARGET_WORDS[227], Word::try_from("those").unwrap())
    }
}
