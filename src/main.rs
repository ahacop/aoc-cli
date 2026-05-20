mod client;
mod session;

use anyhow::{Context, Result, ensure};
use clap::{Parser, Subcommand};
use std::io::{self, IsTerminal, Read, Write};

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
    /// Find it in your browser after logging in to https://adventofcode.com
    /// (DevTools -> Application -> Cookies -> `session`). If TOKEN is omitted,
    /// the cookie is read from stdin.
    Login { token: Option<String> },
    /// Print the path of the saved session file.
    Where,
    /// Fetch the puzzle description for a given day.
    Puzzle { year: u32, day: u32 },
    /// Fetch the puzzle input for a given day.
    Input { year: u32, day: u32 },
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
        Command::Input { year, day } => {
            validate(year, day)?;
            let token = session::load()?;
            let body = client::fetch_input(year, day, &token)?;
            io::stdout().write_all(body.as_bytes())?;
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
        None => {
            if io::stdin().is_terminal() {
                eprint!("Paste your AoC session cookie: ");
                io::stderr().flush().ok();
            }
            let mut buf = String::new();
            io::stdin()
                .read_to_string(&mut buf)
                .context("reading token from stdin")?;
            buf
        }
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
