//! Shows and their episodes' titles from TVmaze (tvmaze.com), for naming
//! episodes (naming.rs) when there's no TMDB key: it needs no key, so a show
//! gets its proper titles without any setup. Shows only (it has no movies).
//! Asked only when someone looks a show up, and then for that show's episodes;
//! requests carry only what was searched for, or the show chosen. Its data is
//! CC BY-SA, credited in Settings › General.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use serde::Deserialize;
use tauri::State;

use crate::commands::Problem;
use crate::tmdb::{FoundTitle, TitleSource, split_year};

const API: &str = "https://api.tvmaze.com";

/// Episode titles by season and episode.
type Titles = HashMap<(u32, u32), String>;

pub struct Tvmaze {
    http: reqwest::Client,
    /// Each show's titles, by its id: they don't change while the app runs.
    shows: Mutex<HashMap<u64, Titles>>,
}

impl Default for Tvmaze {
    fn default() -> Self {
        let http = reqwest::Client::builder()
            .user_agent(concat!("Transceiver/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(15))
            .build()
            .unwrap_or_default();
        Self { http, shows: Mutex::default() }
    }
}

#[derive(Deserialize)]
struct Found {
    show: Show,
}

#[derive(Deserialize)]
struct Show {
    id: u64,
    name: String,
    #[serde(default)]
    premiered: Option<String>,
    #[serde(default)]
    summary: Option<String>,
}

#[derive(Deserialize)]
struct Episode {
    season: u32,
    /// None for specials.
    number: Option<u32>,
    name: String,
}

impl Tvmaze {
    async fn get<T: for<'de> Deserialize<'de>>(&self, path: &str, query: &[(&str, &str)]) -> Result<T, String> {
        let response = self
            .http
            .get(format!("{API}{path}"))
            .query(query)
            .send()
            .await
            .map_err(|e| format!("Couldn't reach TVmaze: {e}"))?;
        match response.status().as_u16() {
            200..=299 => response.json().await.map_err(|e| format!("TVmaze's answer wasn't readable: {e}")),
            404 => Err("TVmaze doesn't have that.".to_owned()),
            429 => Err("TVmaze asks to slow down; try again in a moment.".to_owned()),
            code => Err(format!("TVmaze answered {code}.")),
        }
    }

    /// Shows called `query`; "Name (1962)" keeps those that started that year, when any did.
    pub async fn search(&self, query: &str) -> Result<Vec<FoundTitle>, String> {
        let (title, year) = split_year(query);
        let found: Vec<Found> = self.get("/search/shows", &[("q", &title)]).await?;
        Ok(found_titles(found, year))
    }

    /// Every episode's title, by season and episode.
    pub async fn titles(&self, show: u64) -> Result<Titles, String> {
        if let Some(titles) = self.shows.lock().unwrap_or_else(|e| e.into_inner()).get(&show) {
            return Ok(titles.clone());
        }
        let episodes: Vec<Episode> = self.get(&format!("/shows/{show}/episodes"), &[]).await?;
        let titles = titles_of(episodes);
        self.shows.lock().unwrap_or_else(|e| e.into_inner()).insert(show, titles.clone());
        Ok(titles)
    }
}

fn found_titles(found: Vec<Found>, year: Option<u32>) -> Vec<FoundTitle> {
    let all: Vec<FoundTitle> = found
        .into_iter()
        .map(|f| FoundTitle {
            id: f.show.id,
            name: f.show.name,
            year: f.show.premiered.as_deref().and_then(|d| d.get(..4)).and_then(|y| y.parse().ok()),
            overview: f.show.summary.as_deref().map(plain_text).unwrap_or_default(),
            source: TitleSource::Tvmaze,
        })
        .collect();
    match year {
        Some(y) if all.iter().any(|t| t.year == Some(y)) => all.into_iter().filter(|t| t.year == Some(y)).collect(),
        _ => all,
    }
}

fn titles_of(episodes: Vec<Episode>) -> Titles {
    episodes
        .into_iter()
        .filter(|e| !crate::naming::is_placeholder_title(&e.name))
        .filter_map(|e| Some(((e.season, e.number?), e.name.trim().to_owned())))
        .collect()
}

/// A summary's HTML (`<p>A <b>family</b> &amp; …</p>`) as plain text.
fn plain_text(html: &str) -> String {
    let mut text = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                text.push(' ');
            }
            c if !in_tag => text.push(c),
            _ => {}
        }
    }
    let text = text
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ");
    text.split_whitespace().collect::<Vec<_>>().join(" ").replace(" ,", ",").replace(" .", ".")
}

/// Shows on TVmaze (no key needed).
#[tauri::command]
pub async fn tvmaze_search(query: String, tvmaze: State<'_, Tvmaze>) -> Result<Vec<FoundTitle>, Problem> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }
    tvmaze.search(&query).await.map_err(Problem::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_results() {
        let json = r#"[
            {"score": 0.9, "show": {"id": 1, "name": "The Beverly Hillbillies", "premiered": "1962-09-26",
              "summary": "<p>A <b>family</b> strikes oil &amp; moves west.</p>"}},
            {"score": 0.5, "show": {"id": 2, "name": "The Beverly Hillbillies", "premiered": null, "summary": null}}
        ]"#;
        let found = found_titles(serde_json::from_str(json).unwrap(), None);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].year, Some(1962));
        assert_eq!(found[0].overview, "A family strikes oil & moves west.");
        assert_eq!(found[1].year, None);
        assert_eq!(found[0].source, TitleSource::Tvmaze);

        let found = found_titles(serde_json::from_str(json).unwrap(), Some(1962));
        assert_eq!(found.iter().map(|f| f.id).collect::<Vec<_>>(), [1], "the year narrows it");
        let found = found_titles(serde_json::from_str(json).unwrap(), Some(1993));
        assert_eq!(found.len(), 2, "a year nothing matches doesn't empty it");
    }

    #[test]
    fn episodes() {
        let json = r#"[
            {"season": 1, "number": 1, "name": "The Clampetts Strike Oil"},
            {"season": 1, "number": 2, "name": "Getting Settled "},
            {"season": 1, "number": null, "name": "A Special"},
            {"season": 2, "number": 1, "name": ""},
            {"season": 2, "number": 2, "name": "Episode 2"}
        ]"#;
        let titles = titles_of(serde_json::from_str(json).unwrap());
        assert_eq!(titles.len(), 2);
        assert_eq!(titles[&(1, 2)], "Getting Settled");
    }
}
