use crate::client;
use anyhow::{Context, Result, bail, ensure};
use rusqlite::Connection;
use std::collections::HashSet;
use std::io::{self, BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::{cmp, fs};

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
    let mut found: Vec<(u64, String)> = Vec::new();
    for db in firefox_dbs() {
        match read_firefox_copy(&db) {
            Ok(rows) => found.extend(rows),
            Err(_) => {
                if let Ok(extra) = rookie::firefox_based(db, domains.clone()) {
                    cookies.extend(extra);
                }
            }
        }
    }
    found.extend(
        cookies
            .into_iter()
            .filter(|c| c.name == "session")
            .map(|c| (c.expires.unwrap_or(0), c.value)),
    );
    found.retain(|(_, value)| !value.is_empty() && is_hex(value));
    found.sort_by_key(|(expires, _)| cmp::Reverse(*expires));
    let mut seen = HashSet::new();
    found
        .into_iter()
        .map(|(_, value)| value)
        .filter(|value| seen.insert(value.clone()))
        .collect()
}

/// Reads the AoC session cookies from a Firefox cookie DB as `(expiry, value)`
/// pairs, including cookies that exist only in the DB's write-ahead log.
///
/// A running Firefox locks `cookies.sqlite` and keeps recent writes in
/// `cookies.sqlite-wal`. `rookie` gets past the lock with SQLite's `immutable`
/// flag, which also ignores that log, so it can miss a recent login. Opening a
/// private copy of both files needs no such flag, and SQLite applies the log.
///
/// The two files are not copied in one step, so a write between the copies can
/// produce a stale cookie. `obtain_login` tests each cookie against AoC, which
/// rejects a stale one.
fn read_firefox_copy(db: &Path) -> Result<Vec<(u64, String)>> {
    let dir = tempfile::tempdir().context("creating a temporary directory")?;
    let copy = dir.path().join("cookies.sqlite");
    fs::copy(db, &copy).with_context(|| format!("copying {}", db.display()))?;

    let mut wal = db.as_os_str().to_owned();
    wal.push("-wal");
    match fs::copy(&wal, dir.path().join("cookies.sqlite-wal")) {
        Ok(_) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e).context("copying the write-ahead log"),
    }

    let conn = Connection::open(&copy).context("opening the copied cookie DB")?;
    let mut stmt = conn.prepare(
        "SELECT expiry, value FROM moz_cookies
         WHERE host LIKE '%adventofcode.com' AND name = 'session'",
    )?;
    let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    rows.collect::<rusqlite::Result<_>>()
        .context("reading the copied cookie DB")
}

/// Firefox-family cookie DBs in every profile under the known locations. This
/// covers the XDG layout `~/.config/mozilla/firefox/<profile>/` used by NixOS
/// and Home Manager Firefox setups, which `rookie`'s default config misses.
fn firefox_dbs() -> Vec<PathBuf> {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return Vec::new();
    };
    let bases = [
        ".config/mozilla/firefox",
        ".config/librewolf",
        ".config/zen",
        ".mozilla/firefox",
        ".librewolf",
        ".zen",
        "snap/firefox/common/.mozilla/firefox",
        "Library/Application Support/Firefox/Profiles",
    ];
    let mut out = Vec::new();
    for base in bases {
        let Ok(entries) = fs::read_dir(home.join(base)) else {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a cookie DB in the state a running Firefox leaves it: exclusively
    /// locked, with its rows only in the write-ahead log. The connection must
    /// stay open, because closing it moves the rows into the main file.
    fn locked_db_with_wal(path: &Path) -> Connection {
        let conn = Connection::open(path).unwrap();
        conn.execute_batch(
            "PRAGMA locking_mode = EXCLUSIVE;
             PRAGMA journal_mode = WAL;
             PRAGMA wal_autocheckpoint = 0;
             CREATE TABLE moz_cookies (host TEXT, name TEXT, value TEXT, expiry INTEGER);
             INSERT INTO moz_cookies VALUES
               ('.adventofcode.com', 'session', 'abc123', 1800000000),
               ('.adventofcode.com', 'other', 'nope', 1800000000),
               ('.example.com', 'session', 'nope', 1800000000);",
        )
        .unwrap();
        conn
    }

    #[test]
    fn read_firefox_copy_sees_rows_only_in_the_wal() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("cookies.sqlite");
        let _firefox = locked_db_with_wal(&db);

        let wal = dir.path().join("cookies.sqlite-wal");
        assert!(fs::metadata(&wal).unwrap().len() > 0);

        let rows = read_firefox_copy(&db).unwrap();
        assert_eq!(rows, vec![(1800000000, "abc123".to_string())]);
    }

    #[test]
    fn read_firefox_copy_works_without_a_wal() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("cookies.sqlite");
        let conn = Connection::open(&db).unwrap();
        conn.execute_batch(
            "CREATE TABLE moz_cookies (host TEXT, name TEXT, value TEXT, expiry INTEGER);
             INSERT INTO moz_cookies VALUES ('adventofcode.com', 'session', 'abc123', 5);",
        )
        .unwrap();
        drop(conn);

        let rows = read_firefox_copy(&db).unwrap();
        assert_eq!(rows, vec![(5, "abc123".to_string())]);
    }

    #[test]
    fn read_firefox_copy_fails_on_a_missing_db() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read_firefox_copy(&dir.path().join("cookies.sqlite")).is_err());
    }
}
