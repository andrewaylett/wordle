use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::io;
use std::io::{BufRead, stdout, Write};
use std::str::FromStr;
use structopt::StructOpt;
use wordle::words::{EXTENDED_WORDS, TARGET_WORDS};
use wordle::{GuessStatus, Word, WordError, WordGuess};

use rayon::prelude::*;

#[derive(Debug, StructOpt)]
enum Opt {
    FilterFromGuess(FilterFromGuessOpt),
    Analyse(AnalyseOpt),
}

#[derive(Debug, StructOpt)]
struct FilterFromGuessOpt {
    word: String,
    guess: String,
    #[structopt(short = "x", long)]
    extend: bool,
}

#[derive(Debug, StructOpt)]
struct AnalyseOpt {
    #[structopt(short = "x", long)]
    extend: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();
    match opt {
        Opt::FilterFromGuess(opt) => {
            let word = Word::try_from(opt.word.as_str())?;
            let guess = GuessStatus::try_from(opt.guess.as_str())?;
            let iter = TARGET_WORDS.into_iter();
            let mut results: Vec<Word> = if opt.extend {
                iter.chain(EXTENDED_WORDS.into_iter())
                    .filter(|target| {
                        let wg = WordGuess::guess(word, *target);
                        wg.status == guess
                    })
                    .collect()
            } else {
                iter.filter(|target| {
                    let wg = WordGuess::guess(word, *target);
                    wg.status == guess
                })
                .collect()
            };
            results.sort();
            results.iter().for_each(|w| println!("{:?}", w));
        }
        Opt::Analyse(opt) => {
            let stdin = io::stdin();
            let mut lines = stdin.lock().lines();
            let first = lines.next().ok_or(WordError::NotWordle)??;
            let mut parse = first.split(' ');
            let wordle = parse.next().ok_or(WordError::NotWordle)?;
            if wordle != "Wordle" {
                return Err(WordError::NotWordle.into());
            }
            let puzzle_number = parse.next().ok_or(WordError::NotWordle)?;
            let puzzle_number = usize::from_str(puzzle_number)?;
            let target = TARGET_WORDS[puzzle_number];

            let maybe_first_guess: Vec<Result<GuessStatus, anyhow::Error>> = parse
                .next()
                .map(|rest| {
                    if rest.contains(':') {
                        let g = GuessStatus::try_from(rest.trim_start_matches(|char| char != ':'));
                        match g {
                            Ok(g) => vec![Ok(g)],
                            _ => vec![],
                        }
                    } else {
                        vec![]
                    }
                })
                .unwrap_or_else(std::vec::Vec::new);

            let guesses = lines
                .filter(|line| line.as_ref().map(|l| !l.is_empty()).unwrap_or(true))
                .map(|line| {
                    line.map(|line| GuessStatus::try_from(line.as_str()))?
                        .map_err(Into::into) as anyhow::Result<GuessStatus>
                });
            let mut guesses = maybe_first_guess.into_iter().chain(guesses);

            let all_words: BTreeSet<Word> = TARGET_WORDS
                .iter()
                .chain(EXTENDED_WORDS.iter())
                .copied()
                .collect();

            struct RowAnalysis {
                guess: GuessStatus,
                possible_guesses: BTreeSet<Word>,
                possible_targets: BTreeMap<Vec<Word>, BTreeSet<Word>>,
            }

            let initial_guess_chain = BTreeMap::from([(
                vec![],
                if opt.extend {
                    BTreeSet::from_iter(all_words.iter().copied())
                } else {
                    BTreeSet::from_iter(TARGET_WORDS.iter().copied())
                },
            )]);

            let possible_words = guesses.try_fold(vec![], |mut acc: Vec<RowAnalysis>, guess| {
                let guess = guess?;
                let possible_guesses: BTreeSet<Word> = all_words
                    .iter()
                    .filter(|&w| WordGuess::guess(*w, target).status == guess)
                    .copied()
                    .collect();

                let guess_chains: &BTreeMap<Vec<Word>, BTreeSet<Word>> = acc
                    .last()
                    .map(|r| &r.possible_targets)
                    .unwrap_or(&initial_guess_chain);

                let possible_targets: BTreeMap<Vec<Word>, BTreeSet<Word>> = guess_chains
                    .par_iter()
                    .flat_map_iter(|(chain, words)| {
                        possible_guesses.iter().map(move |&word| {
                            let mut new_chain = chain.clone();
                            new_chain.push(word);
                            let new_set: BTreeSet<Word> = words
                                .iter()
                                .filter(|&target| {
                                    let wg = WordGuess::guess_from(word, target);
                                    wg.status == guess
                                })
                                .copied()
                                .collect();
                            (new_chain, new_set)
                        })
                    })
                    .collect();

                print!(".");
                stdout().flush()?;
                acc.push(RowAnalysis {
                    guess,
                    possible_guesses,
                    possible_targets,
                });
                Ok(acc) as Result<Vec<RowAnalysis>, anyhow::Error>
            });

            println!();
            possible_words?
                .iter()
                .for_each(|row| {
                    let guess = &row.guess;
                    let possible = &row.possible_guesses;
                    let targets = &row.possible_targets;
                    let minimum = targets.iter().min_by_key(|(_, a)|{a.len()});
                    let maximum = targets.iter().max_by_key(|(_, a)|{a.len()});
                    let (min_path, min_words) = minimum.unwrap();
                    let (max_path, max_words) = maximum.unwrap();
                    println!(
                        "Guess resulting in {} has {} possible guess{} for between {} and {} targets left, guessing {:?} and {:?} respectively.",
                        guess,
                        possible.len(),
                        if possible.len() != 1 { "es" } else { "" },
                        min_words.len(),
                        max_words.len(),
                        min_path,
                        max_path,
                    )
                })
        }
    }
    Ok(())
}
