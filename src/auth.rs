use crate::client;
use anyhow::{Context, Result, bail, ensure};
use std::cmp;
use std::collections::HashSet;
use std::io::{self, BufRead, IsTerminal, Write};
use std::path::PathBuf;

const LOGIN_URL: &str = "https://adventofcode.com/auth/login";

/// A session token that AoC accepts, and the account it belongs to.
pub struct Login {
    pub token: String,
    pub user: String,
}

/// Finds a working session token: the first browser cookie that AoC accepts,
/// or else one that the user pastes.
pub fn obtain_login() -> Result<Login> {
    let tokens = browser_tokens();
    for token in &tokens {
        if let Some(user) = client::current_user(token)? {
            eprintln!("Found a logged-in AoC session cookie in your browser.");
            return Ok(Login {
                token: token.clone(),
                user,
            });
        }
    }
    if tokens.is_empty() {
        eprintln!("Couldn't read an AoC session cookie from any local browser.");
    } else {
        eprintln!("Found AoC session cookies in your browser, but none of them is logged in.");
    }
    check_token(&prompt_paste()?)
}

/// Checks the format of `raw`, then asks AoC whether it is logged in.
pub fn check_token(raw: &str) -> Result<Login> {
    let token = raw.trim();
    ensure!(!token.is_empty(), "empty session token");
    ensure!(is_hex(token), "session cookie should be a hex string");
    match client::current_user(token)? {
        Some(user) => Ok(Login {
            token: token.to_string(),
            user,
        }),
        None => bail!("AoC doesn't accept this session cookie — it is invalid or expired"),
    }
}

fn is_hex(token: &str) -> bool {
    token.chars().all(|c| c.is_ascii_hexdigit())
}

/// Every distinct AoC session cookie in the local browsers, latest expiry first
/// so that the most recently refreshed login is tried first.
fn browser_tokens() -> Vec<String> {
    let domains = Some(vec!["adventofcode.com".to_string()]);
    let mut cookies = rookie::load(domains.clone()).unwrap_or_default();
    for db in extra_firefox_dbs() {
        if let Ok(extra) = rookie::firefox_based(db, domains.clone()) {
            cookies.extend(extra);
        }
    }
    let mut found: Vec<(u64, String)> = cookies
        .into_iter()
        .filter(|c| c.name == "session")
        .map(|c| (c.expires.unwrap_or(0), c.value))
        .collect();
    found.retain(|(_, value)| !value.is_empty() && is_hex(value));
    found.sort_by_key(|(expires, _)| cmp::Reverse(*expires));
    let mut seen = HashSet::new();
    found
        .into_iter()
        .map(|(_, value)| value)
        .filter(|value| seen.insert(value.clone()))
        .collect()
}

/// Firefox cookie DBs in non-standard locations rookie's default config misses
/// (notably the XDG layout `~/.config/mozilla/firefox/<profile>/` used by
/// NixOS and Home Manager Firefox setups).
fn extra_firefox_dbs() -> Vec<PathBuf> {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return Vec::new();
    };
    let bases = [
        home.join(".config/mozilla/firefox"),
        home.join(".config/librewolf"),
        home.join(".config/zen"),
    ];
    let mut out = Vec::new();
    for base in bases {
        let Ok(entries) = std::fs::read_dir(&base) else {
            continue;
        };
        for entry in entries.flatten() {
            let db = entry.path().join("cookies.sqlite");
            if db.is_file() {
                out.push(db);
            }
        }
    }
    out
}

fn prompt_paste() -> Result<String> {
    match webbrowser::open(LOGIN_URL) {
        Ok(_) => eprintln!("\nOpened {LOGIN_URL} in your browser."),
        Err(_) => eprintln!("\nOpen this URL in your browser: {LOGIN_URL}"),
    }
    eprintln!();
    eprintln!("After logging in, copy the `session` cookie's value:");
    eprintln!();
    eprintln!("  Chrome/Edge/Brave: DevTools (F12) → Application tab");
    eprintln!("                     → Storage → Cookies → https://adventofcode.com");
    eprintln!("                     → click `session` → copy Value");
    eprintln!("  Firefox:           DevTools (F12) → Storage tab");
    eprintln!("                     → Cookies → https://adventofcode.com");
    eprintln!("                     → click `session` → copy Value");
    eprintln!("  Safari:            Develop → Show Web Inspector → Storage");
    eprintln!("                     → Cookies → adventofcode.com → session");
    eprintln!();
    eprintln!("The value is a 128-char hex string. Treat it like a password.");
    eprintln!();

    let mut buf = String::new();
    let stdin = io::stdin();
    if stdin.is_terminal() {
        eprint!("Paste session cookie: ");
        io::stderr().flush().ok();
        stdin
            .lock()
            .read_line(&mut buf)
            .context("reading token from stdin")?;
    } else {
        use std::io::Read;
        stdin
            .lock()
            .read_to_string(&mut buf)
            .context("reading token from stdin")?;
    }
    Ok(buf)
}
