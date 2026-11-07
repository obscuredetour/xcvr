//! Shows, movies and episodes' titles from TMDB (themoviedb.org), for naming
//! them (naming.rs). Asked only when someone renames for a media library and
//! looks a show or movie up, with their own TMDB key (Settings › General), kept
//! in the keychain like the RPC passwords (secrets.rs). Requests carry only
//! what was searched for, or the show and season chosen.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::State;
use ts_rs::TS;

use crate::commands::{Problem, with_secrets};
use crate::naming::MediaKind;
use crate::secrets::Secrets;

const API: &str = "https://api.themoviedb.org/3";
/// The keychain account the key is kept under.
pub const KEY_ACCOUNT: &str = "tmdb-api-key";

/// A show or movie found on TMDB.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct TmdbTitle {
    pub id: u64,
    pub name: String,
    /// The year it first aired, or came out.
    pub year: Option<u32>,
    pub overview: String,
}

/// Whether there's a key, and where it's kept.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct TmdbStatus {
    pub has_key: bool,
    pub key_storage: String,
}

/// Seasons' episode titles, by show and season: they don't change while the app runs.
type Seasons = HashMap<(u64, u32), HashMap<u32, String>>;

pub struct Tmdb {
    http: reqwest::Client,
    seasons: Mutex<Seasons>,
}

impl Default for Tmdb {
    fn default() -> Self {
        let http = reqwest::Client::builder()
            .user_agent(concat!("Transceiver/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(15))
            .build()
            .unwrap_or_default();
        Self { http, seasons: Mutex::default() }
    }
}

impl Tmdb {
    /// GET a path with the key: a v4 read access token (a JWT, "eyJ…") as a bearer
    /// token, else a v3 API key as `api_key`.
    async fn get<T: for<'de> Deserialize<'de>>(
        &self,
        key: &str,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<T, String> {
        let mut request = self.http.get(format!("{API}{path}")).query(query);
        request = if key.starts_with("eyJ") { request.bearer_auth(key) } else { request.query(&[("api_key", key)]) };
        let response = request.send().await.map_err(|e| format!("Couldn't reach TMDB: {e}"))?;
        match response.status().as_u16() {
            200..=299 => response.json().await.map_err(|e| format!("TMDB's answer wasn't readable: {e}")),
            401 => Err("TMDB refused the key. Check it in Settings › General.".to_owned()),
            404 => Err("TMDB doesn't have that.".to_owned()),
            429 => Err("TMDB asks to slow down; try again in a moment.".to_owned()),
            code => Err(format!("TMDB answered {code}.")),
        }
    }

    pub async fn search(&self, key: &str, query: &str, kind: MediaKind) -> Result<Vec<TmdbTitle>, String> {
        #[derive(Deserialize)]
        struct Page {
            results: Vec<Found>,
        }
        // Shows have a name and a first air date; movies a title and a release date.
        #[derive(Deserialize)]
        struct Found {
            id: u64,
            #[serde(alias = "title")]
            name: String,
            #[serde(default, alias = "release_date")]
            first_air_date: Option<String>,
            #[serde(default)]
            overview: Option<String>,
        }
        let (title, year) = split_year(query);
        let mut params = vec![("query", title)];
        let (path, year_param) = match kind {
            MediaKind::Show => ("/search/tv", "first_air_date_year"),
            MediaKind::Movie => ("/search/movie", "year"),
        };
        if let Some(year) = year {
            params.push((year_param, year.to_string()));
        }
        let page: Page = self.get(key, path, &params).await?;
        Ok(page
            .results
            .into_iter()
            .map(|f| TmdbTitle {
                id: f.id,
                name: f.name,
                year: f.first_air_date.as_deref().and_then(|d| d.get(..4)).and_then(|y| y.parse().ok()),
                overview: f.overview.unwrap_or_default(),
            })
            .collect())
    }

    /// A season's episode titles, by episode number.
    pub async fn season(&self, key: &str, show: u64, season: u32) -> Result<HashMap<u32, String>, String> {
        if let Some(titles) = self.seasons.lock().unwrap_or_else(|e| e.into_inner()).get(&(show, season)) {
            return Ok(titles.clone());
        }
        #[derive(Deserialize)]
        struct Season {
            episodes: Vec<Ep>,
        }
        #[derive(Deserialize)]
        struct Ep {
            episode_number: u32,
            name: String,
        }
        let found: Season = self.get(key, &format!("/tv/{show}/season/{season}"), &[]).await?;
        let titles: HashMap<u32, String> = found
            .episodes
            .into_iter()
            .filter(|e| !e.name.trim().is_empty())
            .map(|e| (e.episode_number, e.name.trim().to_owned()))
            .collect();
        self.seasons.lock().unwrap_or_else(|e| e.into_inner()).insert((show, season), titles.clone());
        Ok(titles)
    }

    /// Titles for every season asked for, by season and episode.
    pub async fn titles(
        &self,
        key: &str,
        show: u64,
        seasons: impl IntoIterator<Item = u32>,
    ) -> Result<HashMap<(u32, u32), String>, String> {
        let mut all = HashMap::new();
        for season in seasons {
            for (episode, title) in self.season(key, show, season).await? {
                all.insert((season, episode), title);
            }
        }
        Ok(all)
    }
}

/// "The Beverly Hillbillies (1962)" → the title and the year, to search by both.
fn split_year(query: &str) -> (String, Option<u32>) {
    let q = query.trim();
    if let Some(open) = q.rfind('(')
        && q.ends_with(')')
        && let Ok(year) = q[open + 1..q.len() - 1].trim().parse::<u32>()
        && (1900..2100).contains(&year)
    {
        return (q[..open].trim().to_owned(), Some(year));
    }
    (q.to_owned(), None)
}

/// The key, when there is one.
pub async fn key(secrets: &Arc<Secrets>) -> Result<Option<String>, Problem> {
    with_secrets(secrets, |s| s.get(KEY_ACCOUNT)).await.map_err(Problem::from)
}

#[tauri::command]
pub async fn tmdb_status(secrets: State<'_, Arc<Secrets>>) -> Result<TmdbStatus, Problem> {
    with_secrets(&secrets, |s| Ok(TmdbStatus { has_key: s.get(KEY_ACCOUNT)?.is_some(), key_storage: s.place() }))
        .await
        .map_err(Problem::from)
}

/// Save a key (checked with TMDB first), or forget it with None.
#[tauri::command]
pub async fn set_tmdb_key(
    key: Option<String>,
    secrets: State<'_, Arc<Secrets>>,
    tmdb: State<'_, Tmdb>,
) -> Result<TmdbStatus, Problem> {
    match key.map(|k| k.trim().to_owned()).filter(|k| !k.is_empty()) {
        None => with_secrets(&secrets, |s| s.delete(KEY_ACCOUNT)).await.map_err(Problem::from)?,
        Some(key) => {
            #[derive(Deserialize)]
            struct Answer {}
            tmdb.get::<Answer>(&key, "/authentication", &[]).await.map_err(Problem::from)?;
            with_secrets(&secrets, move |s| s.set(KEY_ACCOUNT, &key)).await.map_err(Problem::from)?;
        }
    }
    tmdb_status(secrets).await
}

#[tauri::command]
pub async fn tmdb_search(
    query: String,
    kind: MediaKind,
    secrets: State<'_, Arc<Secrets>>,
    tmdb: State<'_, Tmdb>,
) -> Result<Vec<TmdbTitle>, Problem> {
    let Some(key) = key(&secrets).await? else {
        return Err(Problem::from("Add a TMDB key in Settings › General to look things up.".to_owned()));
    };
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }
    tmdb.search(&key, &query, kind).await.map_err(Problem::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn years_in_queries() {
        assert_eq!(split_year("The Beverly Hillbillies (1962)"), ("The Beverly Hillbillies".to_owned(), Some(1962)));
        assert_eq!(split_year("Doctor Who"), ("Doctor Who".to_owned(), None));
        assert_eq!(split_year("Show (US)"), ("Show (US)".to_owned(), None));
    }
}
