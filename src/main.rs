use std::error::Error;
use structopt::StructOpt;
use wordle::words::TARGET_WORDS;
use wordle::{GuessStatus, Word, WordGuess};

#[derive(Debug, StructOpt)]
struct Opt {
    word: String,
    guess: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();
    let word = Word::try_from(opt.word.as_str())?;
    let guess = GuessStatus::try_from(opt.guess.as_str())?;
    for target in TARGET_WORDS {
        let target = Word::try_from(target)?;
        let wg = WordGuess::guess(word, target);
        if wg.status == guess {
            println!("{}", target);
        }
    }
    Ok(())
}
