use anyhow::{Context, Result};
use std::io::{self, BufRead, IsTerminal, Write};
use std::path::PathBuf;

const LOGIN_URL: &str = "https://adventofcode.com/auth/login";

pub fn obtain_token() -> Result<String> {
    if let Some(token) = try_browsers() {
        eprintln!("Found AoC session cookie in your browser.");
        return Ok(token);
    }
    eprintln!("Couldn't read an AoC session cookie from any local browser.");
    prompt_paste()
}

fn try_browsers() -> Option<String> {
    let domains = Some(vec!["adventofcode.com".to_string()]);
    let mut cookies = rookie::load(domains.clone()).unwrap_or_default();
    for db in extra_firefox_dbs() {
        if let Ok(extra) = rookie::firefox_based(db, domains.clone()) {
            cookies.extend(extra);
        }
    }
    cookies.retain(|c| c.name == "session" && !c.value.is_empty());
    // Prefer the cookie with the latest expiry — most recently refreshed login.
    cookies.sort_by_key(|c| std::cmp::Reverse(c.expires.unwrap_or(0)));
    cookies.into_iter().next().map(|c| c.value)
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
    eprintln!("The value is a ~96-char hex string. Treat it like a password.");
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
