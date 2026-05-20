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

fn get(url: &str, token: &str) -> Result<String> {
    let resp = client(token)?
        .get(url)
        .send()
        .with_context(|| format!("GET {url}"))?;
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

pub fn fetch_puzzle(year: u32, day: u32, token: &str) -> Result<String> {
    get(&format!("https://adventofcode.com/{year}/day/{day}"), token)
}

pub fn fetch_input(year: u32, day: u32, token: &str) -> Result<String> {
    get(
        &format!("https://adventofcode.com/{year}/day/{day}/input"),
        token,
    )
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
}
