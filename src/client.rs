use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use reqwest::{StatusCode, header, redirect};
use std::time::Duration;

const USER_AGENT: &str = concat!(
    "aoc-cli/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/ahacop/aoc-cli)"
);

fn client(token: &str) -> Result<Client> {
    let cookie = format!("session={token}");
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::COOKIE,
        header::HeaderValue::from_str(&cookie).context("invalid session token")?,
    );
    Client::builder()
        .user_agent(USER_AGENT)
        .default_headers(headers)
        .redirect(redirect::Policy::none())
        .timeout(Duration::from_secs(30))
        .build()
        .context("building HTTP client")
}

fn handle(resp: reqwest::blocking::Response, url: &str) -> Result<String> {
    match resp.status() {
        s if s.is_success() => Ok(resp.text()?),
        StatusCode::FOUND | StatusCode::UNAUTHORIZED => {
            bail!("authentication failed — session cookie missing, invalid, or expired")
        }
        StatusCode::NOT_FOUND => bail!("not found: {url} (is the puzzle out yet?)"),
        s => {
            let body = resp.text().unwrap_or_default();
            bail!("HTTP {s} for {url}\n{}", body.trim())
        }
    }
}

fn get(url: &str, token: &str) -> Result<String> {
    let resp = client(token)?
        .get(url)
        .send()
        .with_context(|| format!("GET {url}"))?;
    handle(resp, url)
}

pub fn fetch_puzzle(year: u32, day: u32, token: &str) -> Result<String> {
    get(&format!("https://adventofcode.com/{year}/day/{day}"), token)
}

pub fn fetch_input(year: u32, day: u32, token: &str) -> Result<String> {
    get(
        &format!("https://adventofcode.com/{year}/day/{day}/input"),
        token,
    )
}

pub fn fetch_calendar(year: u32, token: &str) -> Result<String> {
    get(&format!("https://adventofcode.com/{year}"), token)
}

/// Lowest day on the calendar page that still needs work, paired with the next
/// part to tackle (2 if part 1 is done, otherwise 1). `None` once every released
/// day has both stars, or before any day has been released.
pub fn next_unsolved(html: &str) -> Option<(u32, u8)> {
    for day in 1..=25u32 {
        let label = find_day_label(html, day)?;
        if label.contains("two stars") {
            continue;
        }
        let part = if label.contains("one star") { 2 } else { 1 };
        return Some((day, part));
    }
    None
}

/// Stars earned on each released day of the calendar page, in day order.
/// Stops at the first unreleased day, so `len()` is the count of released days.
pub fn star_summary(html: &str) -> Vec<u8> {
    let mut stars = Vec::with_capacity(25);
    for day in 1..=25u32 {
        let Some(label) = find_day_label(html, day) else {
            break;
        };
        let count = if label.contains("two stars") {
            2
        } else if label.contains("one star") {
            1
        } else {
            0
        };
        stars.push(count);
    }
    stars
}

fn find_day_label(html: &str, day: u32) -> Option<&str> {
    let prefix = format!("aria-label=\"Day {day}");
    let mut search_pos = 0;
    loop {
        let rel = html[search_pos..].find(&prefix)?;
        let abs = search_pos + rel;
        let after = &html[abs + prefix.len()..];
        // Disambiguate "Day 1" from "Day 10".."Day 19": the next char must end the number.
        match after.as_bytes().first() {
            Some(b'"') | Some(b',') => {
                let close = after.find('"')?;
                return Some(&after[..close]);
            }
            _ => search_pos = abs + prefix.len(),
        }
    }
}

pub fn submit_answer(year: u32, day: u32, part: u8, answer: &str, token: &str) -> Result<String> {
    let url = format!("https://adventofcode.com/{year}/day/{day}/answer");
    let level = part.to_string();
    let resp = client(token)?
        .post(&url)
        .form(&[("level", level.as_str()), ("answer", answer)])
        .send()
        .with_context(|| format!("POST {url}"))?;
    handle(resp, &url)
}

pub fn render_puzzle(html: &str) -> String {
    let articles = extract_articles(html);
    let chunks: Vec<&str> = if articles.is_empty() {
        vec![html]
    } else {
        articles
    };
    chunks
        .iter()
        .map(|c| html2text::from_read(c.as_bytes(), 100))
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub fn render_response(html: &str) -> String {
    let target = extract_first_article(html).unwrap_or(html);
    html2text::from_read(target.as_bytes(), 100)
}

fn extract_articles(html: &str) -> Vec<&str> {
    let open = "<article class=\"day-desc\">";
    let close = "</article>";
    let mut out = Vec::new();
    let mut rest = html;
    while let Some(start) = rest.find(open) {
        rest = &rest[start..];
        let Some(end_rel) = rest.find(close) else {
            break;
        };
        let end = end_rel + close.len();
        out.push(&rest[..end]);
        rest = &rest[end..];
    }
    out
}

fn extract_first_article(html: &str) -> Option<&str> {
    let start = html.find("<article")?;
    let rest = &html[start..];
    let close = "</article>";
    let end_rel = rest.find(close)?;
    Some(&rest[..end_rel + close.len()])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_multiple_articles() {
        let html = "noise<article class=\"day-desc\">one</article>middle\
                    <article class=\"day-desc\">two</article>tail";
        let got = extract_articles(html);
        assert_eq!(got.len(), 2);
        assert!(got[0].contains("one"));
        assert!(got[1].contains("two"));
    }

    #[test]
    fn no_articles_returns_empty() {
        assert!(extract_articles("<p>nothing here</p>").is_empty());
    }

    #[test]
    fn extracts_first_article_without_class() {
        let html = "<main><article><p>That's the right answer!</p></article>tail</main>";
        let got = extract_first_article(html).unwrap();
        assert_eq!(got, "<article><p>That's the right answer!</p></article>");
    }

    #[test]
    fn extract_first_article_none_when_missing() {
        assert!(extract_first_article("<p>no articles here</p>").is_none());
    }

    #[test]
    fn next_unsolved_skips_two_star_days() {
        let html = r#"
            <a aria-label="Day 1, two stars" class="calendar-day1 calendar-verycomplete"></a>
            <a aria-label="Day 2, one star" class="calendar-day2 calendar-complete"></a>
            <a aria-label="Day 3" class="calendar-day3"></a>
        "#;
        assert_eq!(next_unsolved(html), Some((2, 2)));
    }

    #[test]
    fn next_unsolved_returns_first_fresh_day() {
        let html = r#"
            <a aria-label="Day 1, two stars"></a>
            <a aria-label="Day 2, two stars"></a>
            <a aria-label="Day 3"></a>
            <a aria-label="Day 4, one star"></a>
        "#;
        assert_eq!(next_unsolved(html), Some((3, 1)));
    }

    #[test]
    fn next_unsolved_none_when_all_two_stars() {
        let mut html = String::new();
        for d in 1..=25 {
            html.push_str(&format!(r#"<a aria-label="Day {d}, two stars"></a>"#));
        }
        assert_eq!(next_unsolved(&html), None);
    }

    #[test]
    fn next_unsolved_disambiguates_day_one_from_day_ten() {
        // Day 10 appears before Day 1 in the source — naive substring search would
        // pick "Day 10" when looking for "Day 1".
        let html = r#"
            <a aria-label="Day 10, two stars"></a>
            <a aria-label="Day 1"></a>
        "#;
        assert_eq!(next_unsolved(html), Some((1, 1)));
    }

    #[test]
    fn star_summary_counts_stars_per_day() {
        let html = r#"
            <a aria-label="Day 1, two stars"></a>
            <a aria-label="Day 2, one star"></a>
            <a aria-label="Day 3"></a>
        "#;
        assert_eq!(star_summary(html), vec![2, 1, 0]);
    }

    #[test]
    fn star_summary_stops_at_unreleased_days() {
        let html = r#"
            <a aria-label="Day 1, two stars"></a>
            <a aria-label="Day 2, one star"></a>
        "#;
        assert_eq!(star_summary(html), vec![2, 1]);
    }

    #[test]
    fn star_summary_disambiguates_day_one_from_day_ten() {
        let html = r#"
            <a aria-label="Day 10, two stars"></a>
            <a aria-label="Day 1, one star"></a>
        "#;
        assert_eq!(star_summary(html), vec![1]);
    }

    #[test]
    fn next_unsolved_stops_at_unreleased_day() {
        // Only days 1..=3 exist on the calendar (active year mid-December).
        let html = r#"
            <a aria-label="Day 1, two stars"></a>
            <a aria-label="Day 2, two stars"></a>
            <a aria-label="Day 3, two stars"></a>
        "#;
        assert_eq!(next_unsolved(html), None);
    }
}
