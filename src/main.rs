mod auth;
mod cache;
mod client;
mod session;

use anyhow::{Context, Result, bail, ensure};
use clap::{Parser, Subcommand};
use std::io::{self, Read, Write};

#[derive(Parser)]
#[command(name = "aoc", version, about = "Advent of Code CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Store your Advent of Code session cookie.
    ///
    /// With no TOKEN, tries to read the `session` cookie from a local browser
    /// (Chrome, Firefox, Safari, Edge, Brave). If that fails, opens the AoC
    /// login page and prompts you to paste it.
    Login { token: Option<String> },
    /// Print the path of the saved session file.
    Where,
    /// Fetch the puzzle description for a given day.
    Puzzle { year: u32, day: u32 },
    /// Fetch the puzzle input for a given day. Cached on disk after the first fetch.
    Input {
        year: u32,
        day: u32,
        /// Bypass and overwrite the local cache.
        #[arg(long)]
        refresh: bool,
    },
    /// Submit an answer for a given day and part. If ANSWER is omitted, it is read from stdin.
    Submit {
        year: u32,
        day: u32,
        part: u8,
        answer: Option<String>,
    },
    /// Print the next day and part to tackle for YEAR (lowest day not yet two-starred).
    /// Output format: `<day> <part>` on stdout.
    Next { year: u32 },
    /// Print stars earned per day for YEAR plus the total.
    Stars { year: u32 },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Login { token } => login(token),
        Command::Where => {
            println!("{}", session::path()?.display());
            Ok(())
        }
        Command::Puzzle { year, day } => {
            validate(year, day)?;
            let token = session::load()?;
            let html = client::fetch_puzzle(year, day, &token)?;
            io::stdout().write_all(client::render_puzzle(&html).as_bytes())?;
            Ok(())
        }
        Command::Input { year, day, refresh } => {
            validate(year, day)?;
            let cached = if refresh {
                None
            } else {
                cache::read_input(year, day)?
            };
            let body = match cached {
                Some(b) => b,
                None => {
                    let token = session::load()?;
                    let body = client::fetch_input(year, day, &token)?;
                    cache::write_input(year, day, &body)?;
                    body
                }
            };
            io::stdout().write_all(body.as_bytes())?;
            Ok(())
        }
        Command::Submit {
            year,
            day,
            part,
            answer,
        } => {
            validate(year, day)?;
            ensure!(part == 1 || part == 2, "part must be 1 or 2");
            let raw = match answer {
                Some(a) => a,
                None => {
                    let mut buf = String::new();
                    io::stdin()
                        .read_to_string(&mut buf)
                        .context("reading answer from stdin")?;
                    buf
                }
            };
            let answer = raw.trim();
            ensure!(!answer.is_empty(), "empty answer");
            let token = session::load()?;
            let html = client::submit_answer(year, day, part, answer, &token)?;
            io::stdout().write_all(client::render_response(&html).as_bytes())?;
            Ok(())
        }
        Command::Next { year } => {
            ensure!(year >= 2015, "year must be >= 2015");
            let token = session::load()?;
            let html = client::fetch_calendar(year, &token)?;
            match client::next_unsolved(&html) {
                Some((day, part)) => {
                    println!("{day} {part}");
                    Ok(())
                }
                None => bail!("no puzzles left for {year} — all released days are two-starred"),
            }
        }
        Command::Stars { year } => {
            ensure!(year >= 2015, "year must be >= 2015");
            let token = session::load()?;
            let html = client::fetch_calendar(year, &token)?;
            let stars = client::star_summary(&html);
            if stars.is_empty() {
                bail!("no released days yet for {year}");
            }
            for (i, &s) in stars.iter().enumerate() {
                let day = i as u32 + 1;
                let marks = match s {
                    2 => "**",
                    1 => "*",
                    _ => "",
                };
                println!("Day {day:>2}: {marks}");
            }
            let total: u32 = stars.iter().map(|&s| u32::from(s)).sum();
            let possible = stars.len() as u32 * 2;
            println!("Total: {total}/{possible} stars");
            Ok(())
        }
    }
}

fn validate(year: u32, day: u32) -> Result<()> {
    ensure!(year >= 2015, "year must be >= 2015");
    ensure!((1..=25).contains(&day), "day must be in 1..=25");
    Ok(())
}

fn login(token: Option<String>) -> Result<()> {
    let raw = match token {
        Some(t) => t,
        None => auth::obtain_token()?,
    };
    let token = raw.trim();
    ensure!(!token.is_empty(), "empty session token");
    ensure!(
        token.chars().all(|c| c.is_ascii_hexdigit()),
        "session cookie should be a hex string"
    );
    let path = session::save(token)?;
    eprintln!("Saved session to {}", path.display());
    Ok(())
}
