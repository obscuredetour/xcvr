//! New names for a torrent's folder and files, for a media library, as Plex,
//! Jellyfin and Kodi expect them:
//!
//! - A show: episodes named `Show (Year) - s01e01 - Title.mkv`, its folder (and
//!   season folders inside it) `Season 01`. The season and episode come from each
//!   file's name (`S01E01`, `1x01`, or an episode number inside a `Season 1`
//!   folder); the title from TMDB or TVmaze when the show's chosen there
//!   (tmdb.rs, tvmaze.rs), else
//!   from what follows the episode number in the file's name, up to the release
//!   tags (`1080p`, `WEB-DL`, …).
//! - A movie: its folder `Title (Year)`, the film (its largest video, or its
//!   parts: `cd1`, `part 2`) and its subtitles named the same.
//!
//! Files that aren't episodes or the film (samples, extras, `.nfo`s, artwork)
//! keep their names. Planning is pure (names in, names out), so the add dialog
//! can show it before anything's added; renames.rs carries it out.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// What a torrent holds, as far as naming goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum MediaKind {
    Show,
    Movie,
}

/// How to rename a torrent.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct NameOptions {
    /// The torrent's new name: its folder, or its file when it's one file. None keeps it.
    /// (A movie's folder is named for the movie instead.)
    pub top: Option<String>,
    /// Name episode files after the show; None keeps the files' names.
    pub episodes: Option<EpisodeNames>,
    /// Name the folder and the film after the movie.
    pub movie: Option<MovieName>,
}

/// Episodes' names: `{show} - s01e01 - {title}`.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct EpisodeNames {
    /// As it starts each name, e.g. "The Beverly Hillbillies (1962)".
    pub show: String,
    /// The show on TMDB, for the episodes' titles.
    pub tmdb: Option<u64>,
    /// The show on TVmaze, for the episodes' titles without a TMDB key. With
    /// neither, they come from the file names.
    pub tvmaze: Option<u64>,
}

/// A movie's names: the folder `{title}`, the film `{title}.mkv`.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct MovieName {
    /// "The Matrix (1999)".
    pub title: String,
    /// The movie on TMDB, for `tag`.
    pub tmdb: Option<u64>,
    /// Put its TMDB id in the folder's name (`{tmdb-603}`), so Plex matches the right film.
    pub tag: bool,
    /// For a torrent that's one file: move it into a folder of its own, named for the film.
    pub own_folder: bool,
}

/// What renaming would do.
#[derive(Debug, Clone, Default, PartialEq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct NamePlan {
    /// The torrent's name now.
    pub name: String,
    /// Its new name, when it changes.
    pub top: Option<String>,
    /// One file, not in a folder: its name is the torrent's.
    pub single_file: bool,
    pub files: Vec<PlannedFile>,
    /// Folders inside it to rename (season folders), deepest first.
    pub folders: Vec<PlannedFile>,
    /// A folder to move a one-file torrent into, in its download folder (a movie's own).
    pub own_folder: Option<String>,
    /// What it looks like: episodes (a show), or a film with a year (a movie).
    pub kind: Option<MediaKind>,
    /// How many episode files it has.
    pub episode_count: u32,
    /// How many seasons its episodes are from.
    pub season_count: u32,
    /// The one season its episodes are in, when it has several episodes (for a
    /// `Season 01` folder; a single episode's folder isn't a season's).
    pub season: Option<u32>,
    /// The show, as the torrent's name gives it ("The Beverly Hillbillies (1962)").
    pub guessed_show: Option<String>,
    /// The movie, as the torrent's name gives it ("The Matrix (1999)").
    pub guessed_movie: Option<String>,
    /// How many episodes' titles came from TMDB.
    pub tmdb_titles: u32,
    /// Why TMDB's titles couldn't be had, when they were asked for.
    pub problem: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PlannedFile {
    /// Its path now, as Transmission names it (starting with the torrent's folder).
    pub path: String,
    /// Its new name (the last part), when it changes.
    pub to: Option<String>,
    /// Why it keeps its name, when it's not obvious.
    pub note: Option<String>,
}

/// A file in a torrent: its path as Transmission names it, and its size in bytes.
#[derive(Debug, Clone, PartialEq)]
pub struct TorrentFile {
    pub path: String,
    pub size: i64,
}

/// An episode, as a file's name gives it.
#[derive(Debug, Clone, PartialEq)]
pub struct Episode {
    pub season: u32,
    pub episode: u32,
    /// The last episode of a file holding several (`S01E01E02`).
    pub last: Option<u32>,
    /// The title from the file's name, release tags and all taken off.
    pub title: Option<String>,
}

impl Episode {
    /// `s01e01`, or `s01e01-e02`.
    fn code(&self) -> String {
        let mut code = format!("s{:02}e{:02}", self.season, self.episode);
        if let Some(last) = self.last.filter(|&l| l > self.episode) {
            code.push_str(&format!("-e{last:02}"));
        }
        code
    }

    /// Every episode number the file holds.
    fn numbers(&self) -> impl Iterator<Item = u32> {
        self.episode..=self.last.unwrap_or(self.episode).max(self.episode)
    }
}

const VIDEO: &[&str] = &["mkv", "mp4", "m4v", "avi", "mov", "wmv", "ts", "m2ts", "webm", "mpg", "mpeg", "flv", "ogm"];
const SUBTITLES: &[&str] = &["srt", "ass", "ssa", "sub", "idx", "vtt", "sup", "smi"];
/// Language and kind tags a subtitle's name ends with (`.en.srt`, `.eng.forced.srt`), kept on renaming.
const SUBTITLE_TAGS: &[&str] = &[
    "en",
    "eng",
    "english",
    "fr",
    "fre",
    "fra",
    "french",
    "es",
    "spa",
    "spanish",
    "de",
    "ger",
    "deu",
    "german",
    "it",
    "ita",
    "italian",
    "pt",
    "por",
    "portuguese",
    "pt-br",
    "nl",
    "dut",
    "nld",
    "dutch",
    "ja",
    "jpn",
    "japanese",
    "zh",
    "chi",
    "zho",
    "chinese",
    "ko",
    "kor",
    "korean",
    "ru",
    "rus",
    "russian",
    "sv",
    "swe",
    "swedish",
    "no",
    "nor",
    "nb",
    "norwegian",
    "da",
    "dan",
    "danish",
    "fi",
    "fin",
    "finnish",
    "pl",
    "pol",
    "polish",
    "ar",
    "ara",
    "arabic",
    "he",
    "heb",
    "hebrew",
    "tr",
    "tur",
    "turkish",
    "cs",
    "cze",
    "ces",
    "el",
    "gre",
    "ell",
    "hu",
    "hun",
    "ro",
    "rum",
    "ron",
    "forced",
    "sdh",
    "cc",
    "hi",
    "default",
];
/// Words that start the release tags after a title (lowercase).
const RELEASE_TAGS: &[&str] = &[
    "480p", "576p", "720p", "1080p", "1080i", "2160p", "4k", "uhd", "hdr", "hdr10", "dv", "dovi", "web", "web-dl",
    "webdl", "webrip", "web-rip", "bluray", "blu-ray", "bdrip", "brrip", "bdremux", "remux", "hdtv", "pdtv", "sdtv",
    "dvdrip", "dvd", "dvdr", "hdrip", "x264", "x265", "h264", "h265", "hevc", "avc", "xvid", "divx", "aac", "ac3",
    "eac3", "dts", "truehd", "atmos", "flac", "10bit", "8bit", "amzn", "nf", "dsnp", "hmax", "atvp", "hulu", "pcok",
    "itunes", "proper", "repack", "internal", "subbed", "dubbed", "dl",
];

static SXXEYY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:^|[^a-z0-9])s(\d{1,2})[ ._-]?e(\d{1,3})(?:(?:[ ._]?-[ ._]?e?|e)(\d{1,3}))?(?:[^0-9]|$)")
        .unwrap()
});
static NXNN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:^|[^a-z0-9])(\d{1,2})x(\d{2,3})(?:-(?:\d{1,2}x)?(\d{2,3}))?(?:[^0-9]|$)").unwrap()
});
/// An episode number alone, for files in a season's folder: `E01`, `Episode 1`, `01 - Title`.
static EPISODE_ONLY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:(?:^|[^a-z0-9])(?:e|ep|episode)[ ._-]?(\d{1,3})|^(\d{1,3}))(?:[ ._-]|$)").unwrap()
});
static SEASON: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:^|[^a-z0-9])(?:season|series|saison|staffel|s)[ ._-]?(\d{1,2})(?:[^0-9]|$)").unwrap()
});
/// Where a show's name ends in a torrent's name: its first season or episode.
static SHOW_END: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:^|[^a-z0-9])(?:s\d{1,2}(?:[ ._-]?e\d{1,3})?|season[ ._-]?\d|series[ ._-]?\d|\d{1,2}x\d{2}|complete)(?:[^a-z0-9]|$)")
        .unwrap()
});
static YEAR_AT_END: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(.*?)[ ]\(?((?:19|20)\d\d)\)?$").unwrap());
static BRACKETS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[[^\]]*\]|\{[^}]*\}").unwrap());
/// A year in a film's name: `1999`, `(1999)`.
static YEAR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(?:^|[^a-z0-9])\(?((?:19|20)\d\d)\)?(?:[^a-z0-9]|$)").unwrap());
/// One part of a film in several files: `CD1`, `Part 2`, `pt.1`, `Disc 2`.
static PART: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(?:^|[^a-z0-9])(?:cd|disc|disk|part|pt)[ ._-]?(\d{1,2})(?:[^0-9]|$)").unwrap());
static SAMPLE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)(?:^|[^a-z])sample(?:[^a-z]|$)").unwrap());

/// A file's name split into its stem and extension (lowercase), when it has one.
fn split_ext(name: &str) -> (&str, Option<String>) {
    match name.rsplit_once('.') {
        Some((stem, ext))
            if !stem.is_empty() && (1..=5).contains(&ext.len()) && ext.chars().all(|c| c.is_ascii_alphanumeric()) =>
        {
            (stem, Some(ext.to_ascii_lowercase()))
        }
        _ => (name, None),
    }
}

/// The season a folder's name is for (`Season 1`, `S01`, `Show.S02.1080p`).
fn folder_season(name: &str) -> Option<u32> {
    if SXXEYY.is_match(name) {
        return None; // One episode's folder, not a season's.
    }
    SEASON.captures(name)?.get(1)?.as_str().parse().ok()
}

/// The episode a file's name (without its extension) is, if it's one. `season` is
/// the season its folder is for, for names that only number the episode.
pub fn parse_episode(stem: &str, season: Option<u32>) -> Option<Episode> {
    let num = |m: Option<regex::Match<'_>>| m.and_then(|m| m.as_str().parse::<u32>().ok());
    let (season, episode, last, rest) = if let Some(c) = SXXEYY.captures(stem) {
        (num(c.get(1))?, num(c.get(2))?, num(c.get(3)), &stem[c.get(0)?.end()..])
    } else if let Some(c) = NXNN.captures(stem) {
        (num(c.get(1))?, num(c.get(2))?, num(c.get(3)), &stem[c.get(0)?.end()..])
    } else {
        let season = season?;
        let c = EPISODE_ONLY.captures(stem)?;
        (season, num(c.get(1)).or(num(c.get(2)))?, None, &stem[c.get(0)?.end()..])
    };
    // The match ends a character past the number (a separator), which `rest` doesn't need.
    Some(Episode { season, episode, last, title: clean_title(rest) })
}

/// Words with dots or underscores for spaces (`The.Clampetts.Strike.Oil`) as words with spaces.
fn spaced(text: &str) -> String {
    let text = text.replace('_', " ");
    if text.contains(' ') { text } else { text.replace('.', " ") }
}

fn is_release_tag(word: &str) -> bool {
    let w = word.trim_matches(|c: char| !c.is_ascii_alphanumeric()).to_ascii_lowercase();
    if w.is_empty() {
        return false;
    }
    RELEASE_TAGS.contains(&w.as_str())
        || RELEASE_TAGS.contains(&word.to_ascii_lowercase().as_str())
        // DDP5, DD+5, AAC2, H 264 (H.264 spaced), 5 1 (5.1 spaced) are caught by their first word.
        // WEB-DL-GRP: a tag with the group on the end.
        || w.split_once('-').is_some_and(|(first, _)| RELEASE_TAGS.contains(&first) && first.len() > 2)
        // DDP5, AAC2 (DDP5.1 and AAC2.0, spaced).
        || ["ddp", "dd", "aac", "eac3", "ac3", "dts"]
            .iter()
            .any(|t| w.starts_with(t) && w[t.len()..].chars().all(|c| c.is_ascii_digit()))
}

/// The title in what follows a file's episode number: release tags, bracketed
/// tags and the release group taken off, dots made spaces.
fn clean_title(rest: &str) -> Option<String> {
    let rest = BRACKETS.replace_all(rest, " ");
    let scene = !rest.contains(' ');
    let rest = spaced(&rest);
    let mut words: Vec<&str> = Vec::new();
    for word in rest.split_whitespace() {
        if is_release_tag(word) {
            break;
        }
        words.push(word);
    }
    let mut title = words.join(" ");
    // A scene name's group: "Strike.Oil-LOL" (one word, capitals) after the last word.
    if scene
        && let Some((before, group)) = title.rsplit_once('-')
        && !group.is_empty()
        && !before.ends_with(' ')
        && group.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
    {
        title.truncate(before.len());
    }
    let title = title.trim_matches(|c: char| c.is_whitespace() || matches!(c, '-' | '.' | '_' | ',')).to_owned();
    (!title.is_empty()).then_some(title)
}

/// The show a torrent's name is for: what's before its first season or episode, with
/// a year in brackets ("The.Beverly.Hillbillies.1962.S01.1080p" → "The Beverly Hillbillies (1962)").
pub fn guess_show(name: &str) -> Option<String> {
    let (stem, ext) = split_ext(name);
    let name = if ext.is_some_and(|e| VIDEO.contains(&e.as_str())) { stem } else { name };
    let end = SHOW_END.find(name)?.start();
    let show = spaced(&BRACKETS.replace_all(&name[..end], " "));
    let show = show.split_whitespace().collect::<Vec<_>>().join(" ");
    let show = show.trim_matches(|c: char| matches!(c, '-' | '.' | ' ' | '_')).to_owned();
    if show.is_empty() {
        return None;
    }
    Some(match YEAR_AT_END.captures(&show) {
        Some(c) if !c[1].is_empty() => format!("{} ({})", &c[1], &c[2]),
        _ => show,
    })
}

/// A placeholder title ("Episode 1", "TBA"), which a file's own title beats.
pub fn is_placeholder_title(title: &str) -> bool {
    let t = title.trim().to_ascii_lowercase();
    t.is_empty()
        || matches!(t.as_str(), "tba" | "tbd" | "untitled")
        || t.strip_prefix("episode").is_some_and(|n| {
            let n = n.trim_start_matches([' ', '#']);
            !n.is_empty() && n.chars().all(|c| c.is_ascii_digit())
        })
}

/// A name that's safe as a file's on any host: no path separators, and none of the
/// characters Windows shares and SMB refuse.
pub fn safe_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for c in name.chars() {
        match c {
            '/' | '\\' => out.push('-'),
            ':' => out.push_str(" -"),
            '?' | '*' | '"' | '<' | '>' | '|' => {}
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    let out = out.split_whitespace().collect::<Vec<_>>().join(" ");
    out.trim_end_matches(['.', ' ']).to_owned()
}

/// The subtitle's language and kind tags, kept on renaming (`.en.forced`, or
/// `.English` from `2_English.srt`).
fn subtitle_tags(stem: &str) -> String {
    let words: Vec<&str> = stem.split(['.', '_', ' ', '-']).collect();
    let tags: Vec<&str> =
        words.iter().rev().take_while(|w| SUBTITLE_TAGS.contains(&w.to_ascii_lowercase().as_str())).copied().collect();
    // A name that's all tags ("English.srt") is a language, not a title.
    tags.iter().rev().map(|t| format!(".{t}")).collect()
}

fn is_sample(path: &str) -> bool {
    path.split('/').any(|p| SAMPLE.is_match(p))
}

fn ext_of(path: &str) -> Option<String> {
    split_ext(path.rsplit('/').next().unwrap_or(path)).1
}

fn is_video(path: &str) -> bool {
    ext_of(path).is_some_and(|e| VIDEO.contains(&e.as_str()))
}

fn is_subtitle(path: &str) -> bool {
    ext_of(path).is_some_and(|e| SUBTITLES.contains(&e.as_str()))
}

/// The movie a torrent's name is for: what's before its year, with the year in
/// brackets ("The.Matrix.1999.1080p.BluRay" → "The Matrix (1999)"); without a year,
/// what's before the release tags.
pub fn guess_movie(name: &str) -> Option<String> {
    let (stem, ext) = split_ext(name);
    let name = if ext.is_some_and(|e| VIDEO.contains(&e.as_str())) { stem } else { name };
    // The first year with a title before it ("2001.A.Space.Odyssey.1968": 1968).
    for c in YEAR.captures_iter(name) {
        let start = c.get(0)?.start();
        if let Some(title) = clean_title(&name[..start]) {
            return Some(format!("{title} ({})", &c[1]));
        }
    }
    clean_title(name)
}

/// Every episode in a torrent's files, by index, for asking TMDB about their seasons.
pub fn episodes(files: &[TorrentFile]) -> Vec<Option<Episode>> {
    let single = files.len() == 1 && !files[0].path.contains('/');
    files
        .iter()
        .map(|TorrentFile { path, .. }| {
            let parts: Vec<&str> = path.split('/').collect();
            let file = *parts.last()?;
            let (stem, ext) = split_ext(file);
            let ext = ext?;
            if !VIDEO.contains(&ext.as_str()) && !SUBTITLES.contains(&ext.as_str()) {
                return None;
            }
            if parts.iter().any(|p| SAMPLE.is_match(p)) {
                return None;
            }
            // The nearest folder that names a season, the torrent's own included.
            let folders: &[&str] = if single { &[] } else { &parts[..parts.len() - 1] };
            let season = folders.iter().rev().find_map(|f| folder_season(f));
            parse_episode(stem, season)
        })
        .collect()
}

/// What renaming a torrent (`name`, its `files` as Transmission names them) with
/// `options` would do. `titles` are TMDB's, by season and episode.
pub fn plan(
    name: &str,
    files: &[TorrentFile],
    options: &NameOptions,
    titles: &HashMap<(u32, u32), String>,
) -> NamePlan {
    let single_file = files.len() == 1 && !files[0].path.contains('/');
    let found = episodes(files);
    let episode_files: Vec<&Episode> =
        files.iter().zip(&found).filter(|(f, _)| is_video(&f.path)).filter_map(|(_, e)| e.as_ref()).collect();
    let seasons: BTreeSet<u32> = episode_files.iter().map(|e| e.season).collect();
    let guessed_movie = guess_movie(name);
    let mut plan = NamePlan {
        name: name.to_owned(),
        single_file,
        episode_count: episode_files.len() as u32,
        season_count: seasons.len() as u32,
        season: (seasons.len() == 1 && episode_files.len() > 1).then(|| *seasons.first().unwrap()),
        kind: if !episode_files.is_empty() {
            Some(MediaKind::Show)
        } else if guessed_movie.as_deref().is_some_and(|m| m.ends_with(')')) {
            Some(MediaKind::Movie)
        } else {
            None
        },
        guessed_show: guess_show(name)
            .or_else(|| files.iter().find_map(|f| guess_show(f.path.rsplit('/').next().unwrap_or(&f.path)))),
        guessed_movie,
        files: files.iter().map(|f| PlannedFile { path: f.path.clone(), ..PlannedFile::default() }).collect(),
        ..NamePlan::default()
    };

    let top = if let Some(movie) = &options.movie {
        name_movie(&mut plan, files, movie)
    } else {
        if let Some(show) = options.episodes.as_ref().map(|e| safe_name(&e.show)).filter(|s| !s.is_empty()) {
            name_episodes(&mut plan, &found, &show, titles);
            plan.folders = season_folders(files);
        }
        options.top.as_deref().map(safe_name).filter(|t| !t.is_empty())
    };

    // Two files can't share a name in one folder; the second keeps its own.
    let mut taken: HashSet<String> = HashSet::new();
    for file in &mut plan.files {
        let current = file.path.rsplit('/').next().unwrap_or(&file.path);
        if file.to.as_deref() == Some(current) {
            file.to = None;
        }
        if let Some(to) = &file.to {
            let folder = file.path.rsplit_once('/').map_or("", |(f, _)| f);
            if !taken.insert(format!("{folder}/{}", to.to_lowercase())) {
                file.to = None;
                file.note = Some("Another file would have the same name".to_owned());
            }
        }
    }

    plan.top = if single_file {
        // The file is the torrent: its new name, else the name given.
        plan.files[0].to.take().or(top)
    } else {
        top
    }
    .filter(|t| t != name);
    plan
}

/// Episodes' names, and notes on the files that aren't episodes.
fn name_episodes(plan: &mut NamePlan, found: &[Option<Episode>], show: &str, titles: &HashMap<(u32, u32), String>) {
    for (file, episode) in plan.files.iter_mut().zip(found) {
        let current = file.path.rsplit('/').next().unwrap_or(&file.path).to_owned();
        let Some(e) = episode else {
            file.note = Some(
                if !is_video(&file.path) && !is_subtitle(&file.path) {
                    "Not a video"
                } else if is_sample(&file.path) {
                    "A sample"
                } else {
                    "No episode number in its name"
                }
                .to_owned(),
            );
            continue;
        };
        let (stem, ext) = split_ext(&current);
        let ext = ext.unwrap_or_default();
        let tags = if SUBTITLES.contains(&ext.as_str()) { subtitle_tags(stem) } else { String::new() };
        let mut from_tmdb: Vec<&String> = Vec::new();
        for title in e.numbers().filter_map(|n| titles.get(&(e.season, n))) {
            if !from_tmdb.contains(&title) {
                from_tmdb.push(title);
            }
        }
        let title = if from_tmdb.is_empty() {
            e.title.clone()
        } else {
            if VIDEO.contains(&ext.as_str()) {
                plan.tmdb_titles += 1;
            }
            Some(from_tmdb.iter().map(|t| t.as_str()).collect::<Vec<_>>().join(" & "))
        };
        let base = match title.map(|t| safe_name(&t)).filter(|t| !t.is_empty()) {
            Some(t) => format!("{show} - {} - {t}", e.code()),
            None => format!("{show} - {}", e.code()),
        };
        file.to = Some(format!("{base}{tags}.{ext}"));
    }
}

/// Folders inside a torrent that are a season's (`Show.S02.1080p`), renamed `Season 02`;
/// deepest first, so each is renamed before the folder it's in.
fn season_folders(files: &[TorrentFile]) -> Vec<PlannedFile> {
    let mut folders: BTreeSet<String> = BTreeSet::new();
    for f in files {
        let parts: Vec<&str> = f.path.split('/').collect();
        // Below the torrent's own folder (that's `top`), above the file.
        for end in 2..parts.len() {
            folders.insert(parts[..end].join("/"));
        }
    }
    let mut planned: Vec<PlannedFile> = Vec::new();
    let mut taken: HashSet<String> = HashSet::new();
    for path in folders {
        let (parent, current) = path.rsplit_once('/').unwrap_or(("", &path));
        let Some(season) = folder_season(current) else { continue };
        let to = format!("Season {season:02}");
        if to == current {
            continue;
        }
        let mut folder = PlannedFile { path: path.clone(), ..PlannedFile::default() };
        if taken.insert(format!("{parent}/{to}")) {
            folder.to = Some(to);
        } else {
            folder.note = Some("Another folder would have the same name".to_owned());
        }
        planned.push(folder);
    }
    planned.sort_by_key(|f| std::cmp::Reverse(f.path.matches('/').count()));
    planned.retain(|f| f.to.is_some());
    planned
}

/// A movie's names: the film (its largest video, or its parts), its subtitles, and
/// the folder's name (returned, for `top`).
fn name_movie(plan: &mut NamePlan, files: &[TorrentFile], movie: &MovieName) -> Option<String> {
    let title = safe_name(&movie.title);
    if title.is_empty() {
        return None;
    }
    let folder = match movie.tmdb.filter(|_| movie.tag) {
        Some(id) => format!("{title} {{tmdb-{id}}}"),
        None => title.clone(),
    };

    let videos: Vec<usize> =
        (0..files.len()).filter(|&i| is_video(&files[i].path) && !is_sample(&files[i].path)).collect();
    let part_of = |i: usize| {
        let file = files[i].path.rsplit('/').next().unwrap_or(&files[i].path);
        PART.captures(split_ext(file).0).and_then(|c| c[1].parse::<u32>().ok())
    };
    // Parts: two or more videos numbered differently, each at least half the largest's size.
    let largest = videos.iter().map(|&i| files[i].size).max().unwrap_or(0);
    let parts: Vec<(usize, u32)> =
        videos.iter().filter(|&&i| files[i].size * 2 >= largest).filter_map(|&i| Some((i, part_of(i)?))).collect();
    let numbers: BTreeSet<u32> = parts.iter().map(|&(_, n)| n).collect();
    let film: Vec<(usize, Option<u32>)> = if parts.len() >= 2 && numbers.len() == parts.len() {
        parts.into_iter().map(|(i, n)| (i, Some(n))).collect()
    } else {
        videos.iter().copied().max_by_key(|&i| files[i].size).map(|i| (i, None)).into_iter().collect()
    };

    for (i, file) in plan.files.iter_mut().enumerate() {
        let current = files[i].path.rsplit('/').next().unwrap_or(&files[i].path);
        let (stem, ext) = split_ext(current);
        let ext = ext.unwrap_or_default();
        if let Some(&(_, part)) = film.iter().find(|&&(f, _)| f == i) {
            file.to = Some(match part {
                Some(n) => format!("{title} - pt{n}.{ext}"),
                None => format!("{title}.{ext}"),
            });
        } else if is_sample(&file.path) {
            file.note = Some("A sample".to_owned());
        } else if is_subtitle(&file.path) && film.len() == 1 {
            file.to = Some(format!("{title}{}.{ext}", subtitle_tags(stem)));
        } else if is_video(&file.path) {
            file.note = Some("An extra".to_owned());
        } else if !is_subtitle(&file.path) {
            file.note = Some("Not a video".to_owned());
        }
    }

    if plan.single_file && movie.own_folder {
        plan.own_folder = Some(folder.clone());
    }
    Some(folder)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ep(stem: &str) -> Option<(u32, u32, Option<u32>, Option<String>)> {
        parse_episode(stem, None).map(|e| (e.season, e.episode, e.last, e.title))
    }

    fn some(s: &str) -> Option<String> {
        Some(s.to_owned())
    }

    #[test]
    fn episode_numbers_and_titles() {
        assert_eq!(
            ep("The.Beverly.Hillbillies.S01E01.The.Clampetts.Strike.Oil.1080p.WEB-DL.AAC2.0.H.264-GRP"),
            Some((1, 1, None, some("The Clampetts Strike Oil")))
        );
        assert_eq!(ep("The Beverly Hillbillies - 1x02 - Getting Settled"), Some((1, 2, None, some("Getting Settled"))));
        assert_eq!(ep("show_s02e10_some_title_720p"), Some((2, 10, None, some("some title"))));
        assert_eq!(ep("Show.S01E01E02.Pilot.Part.One.HDTV.x264-LOL"), Some((1, 1, Some(2), some("Pilot Part One"))));
        assert_eq!(ep("Show S01E03-E04 Two Parter [1080p]"), Some((1, 3, Some(4), some("Two Parter"))));
        assert_eq!(ep("Show.S03E05.720p.HDTV.x264-DIMENSION"), Some((3, 5, None, None)));
        assert_eq!(ep("Show.S01E01.Pilot-LOL"), Some((1, 1, None, some("Pilot"))));
        assert_eq!(
            ep("Show S01E01 Spider-Man"),
            Some((1, 1, None, some("Spider-Man"))),
            "a hyphen in a spaced title stays"
        );
        assert_eq!(ep("Mr. Robot S01E01 eps1.0_hellofriend"), Some((1, 1, None, some("eps1.0 hellofriend"))));
        assert_eq!(ep("Show.S01E02.Title.WEB-DL-GRP"), Some((1, 2, None, some("Title"))));
        assert_eq!(ep("Movie.2019.1080p.BluRay.x264"), None);
        assert_eq!(ep("Video 1920x1080"), None, "a resolution isn't 1920x108");
    }

    #[test]
    fn episodes_numbered_in_a_season_folder() {
        assert_eq!(
            parse_episode("01 - The Clampetts Strike Oil", Some(1)).map(|e| (e.episode, e.title)),
            Some((1, some("The Clampetts Strike Oil")))
        );
        assert_eq!(parse_episode("Episode 12", Some(2)).map(|e| (e.season, e.episode)), Some((2, 12)));
        assert_eq!(parse_episode("01 - The Clampetts Strike Oil", None), None, "no season, no guessing");
        assert_eq!(folder_season("Season 1"), Some(1));
        assert_eq!(folder_season("The.Beverly.Hillbillies.S01.1080p"), Some(1));
        assert_eq!(folder_season("Show.S01E01.1080p"), None);
        assert_eq!(folder_season("Extras"), None);
    }

    #[test]
    fn shows() {
        assert_eq!(
            guess_show("The.Beverly.Hillbillies.1962.S01.1080p.WEB-DL").as_deref(),
            Some("The Beverly Hillbillies (1962)")
        );
        assert_eq!(
            guess_show("The Beverly Hillbillies (1962) Season 1").as_deref(),
            Some("The Beverly Hillbillies (1962)")
        );
        assert_eq!(guess_show("Show.Name.S01E01.Pilot.mkv").as_deref(), Some("Show Name"));
        assert_eq!(guess_show("[Group] Show - 1x01").as_deref(), Some("Show"));
        assert_eq!(guess_show("ubuntu-24.04-desktop-amd64.iso"), None);
    }

    #[test]
    fn placeholder_titles() {
        assert!(is_placeholder_title("Episode 1"));
        assert!(is_placeholder_title("Episode #12"));
        assert!(is_placeholder_title("TBA"));
        assert!(!is_placeholder_title("Episode One"));
        assert!(!is_placeholder_title("The Clampetts Strike Oil"));
    }

    #[test]
    fn safe_names() {
        assert_eq!(safe_name("Star Trek: The Next Generation"), "Star Trek - The Next Generation");
        assert_eq!(safe_name("AC/DC?  Live."), "AC-DC Live");
    }

    fn files(list: &[&str]) -> Vec<TorrentFile> {
        list.iter().map(|s| TorrentFile { path: (*s).to_owned(), size: 100 }).collect()
    }

    fn episodes_as(show: &str) -> NameOptions {
        NameOptions {
            top: some("Season 01"),
            episodes: Some(EpisodeNames { show: show.to_owned(), ..EpisodeNames::default() }),
            ..NameOptions::default()
        }
    }

    fn movie_as(title: &str) -> NameOptions {
        NameOptions {
            movie: Some(MovieName { title: title.to_owned(), own_folder: true, ..MovieName::default() }),
            ..NameOptions::default()
        }
    }

    fn to(p: &NamePlan) -> Vec<Option<&str>> {
        p.files.iter().map(|f| f.to.as_deref()).collect()
    }

    #[test]
    fn a_season_pack() {
        let name = "The.Beverly.Hillbillies.S01.1080p.WEB-DL-GRP";
        let list = files(&[
            "The.Beverly.Hillbillies.S01.1080p.WEB-DL-GRP/The.Beverly.Hillbillies.S01E01.The.Clampetts.Strike.Oil.1080p.WEB-DL-GRP.mkv",
            "The.Beverly.Hillbillies.S01.1080p.WEB-DL-GRP/The.Beverly.Hillbillies.S01E02.1080p.WEB-DL-GRP.mkv",
            "The.Beverly.Hillbillies.S01.1080p.WEB-DL-GRP/Subs/The.Beverly.Hillbillies.S01E01.The.Clampetts.Strike.Oil.1080p.WEB-DL-GRP.eng.forced.srt",
            "The.Beverly.Hillbillies.S01.1080p.WEB-DL-GRP/Sample/sample.mkv",
            "The.Beverly.Hillbillies.S01.1080p.WEB-DL-GRP/info.nfo",
        ]);
        let titles = HashMap::from([((1, 2), "Getting Settled".to_owned())]);
        let p = plan(name, &list, &episodes_as("The Beverly Hillbillies (1962)"), &titles);
        assert_eq!(p.top.as_deref(), Some("Season 01"));
        assert_eq!(p.season, Some(1));
        assert_eq!(p.guessed_show.as_deref(), Some("The Beverly Hillbillies"));
        assert_eq!(p.tmdb_titles, 1);
        let to: Vec<_> = p.files.iter().map(|f| f.to.as_deref()).collect();
        assert_eq!(
            to,
            [
                Some("The Beverly Hillbillies (1962) - s01e01 - The Clampetts Strike Oil.mkv"),
                Some("The Beverly Hillbillies (1962) - s01e02 - Getting Settled.mkv"),
                Some("The Beverly Hillbillies (1962) - s01e01 - The Clampetts Strike Oil.eng.forced.srt"),
                None,
                None,
            ]
        );
        assert_eq!(p.files[3].note.as_deref(), Some("A sample"));
        assert_eq!(p.files[4].note.as_deref(), Some("Not a video"));
    }

    #[test]
    fn keeping_names() {
        let list = files(&["Folder/S01E01.mkv"]);
        let p = plan("Folder", &list, &NameOptions::default(), &HashMap::new());
        assert_eq!(p.top, None);
        assert!(p.files.iter().all(|f| f.to.is_none() && f.note.is_none()));
    }

    #[test]
    fn one_file_is_the_torrent() {
        let list = files(&["Show.S02E03.Title.720p.mkv"]);
        let p = plan("Show.S02E03.Title.720p.mkv", &list, &episodes_as("Show"), &HashMap::new());
        assert!(p.single_file);
        assert_eq!(p.top.as_deref(), Some("Show - s02e03 - Title.mkv"));
        assert_eq!(p.files[0].to, None);
    }

    #[test]
    fn clashing_names_keep_their_own() {
        let list = files(&["F/Show.S01E01.720p.mkv", "F/Show.S01E01.1080p.mkv"]);
        let p = plan("F", &list, &episodes_as("Show"), &HashMap::new());
        assert_eq!(p.files[0].to.as_deref(), Some("Show - s01e01.mkv"));
        assert_eq!(p.files[1].to, None);
        assert!(p.files[1].note.is_some());
    }

    #[test]
    fn one_episode_in_its_own_folder() {
        let list = files(&["Show.S01E03.1080p/Show.S01E03.Title.1080p.mkv", "Show.S01E03.1080p/info.nfo"]);
        let p = plan("Show.S01E03.1080p", &list, &episodes_as("Show"), &HashMap::new());
        assert_eq!(p.kind, Some(MediaKind::Show));
        assert_eq!(p.season, None, "one episode's folder isn't a season's");
        assert_eq!(to(&p), [Some("Show - s01e03 - Title.mkv"), None]);
    }

    #[test]
    fn season_folders_in_a_series_pack() {
        let list = files(&[
            "Show.Complete/Show.S01.1080p/Show.S01E01.mkv",
            "Show.Complete/Show.S01.1080p/Show.S01E02.mkv",
            "Show.Complete/Season 2/Show.S02E01.mkv",
            "Show.Complete/Show.S03/Extras/Show.S03.Bloopers.mkv",
        ]);
        let options = NameOptions { top: some("Show (2001)"), ..episodes_as("Show (2001)") };
        let p = plan("Show.Complete", &list, &options, &HashMap::new());
        assert_eq!(p.season, None);
        assert_eq!(p.season_count, 2);
        let folders: Vec<_> = p.folders.iter().map(|f| (f.path.as_str(), f.to.as_deref().unwrap())).collect();
        assert_eq!(
            folders,
            [
                ("Show.Complete/Season 2", "Season 02"),
                ("Show.Complete/Show.S01.1080p", "Season 01"),
                ("Show.Complete/Show.S03", "Season 03"),
            ]
        );
        assert_eq!(p.top.as_deref(), Some("Show (2001)"));
    }

    #[test]
    fn movies() {
        assert_eq!(guess_movie("The.Matrix.1999.1080p.BluRay.x264-GRP").as_deref(), Some("The Matrix (1999)"));
        assert_eq!(guess_movie("2001.A.Space.Odyssey.1968.2160p").as_deref(), Some("2001 A Space Odyssey (1968)"));
        assert_eq!(guess_movie("Heat (1995) [1080p]").as_deref(), Some("Heat (1995)"));
        assert_eq!(guess_movie("Night.of.the.Living.Dead.mkv").as_deref(), Some("Night of the Living Dead"));

        let list = vec![
            TorrentFile { path: "The.Matrix.1999.1080p/The.Matrix.1999.1080p.BluRay.mkv".into(), size: 9000 },
            TorrentFile { path: "The.Matrix.1999.1080p/Subs/2_English.srt".into(), size: 1 },
            TorrentFile { path: "The.Matrix.1999.1080p/Featurettes/Making.Of.mkv".into(), size: 900 },
            TorrentFile { path: "The.Matrix.1999.1080p/Sample/sample.mkv".into(), size: 50 },
            TorrentFile { path: "The.Matrix.1999.1080p/poster.jpg".into(), size: 1 },
        ];
        let p = plan("The.Matrix.1999.1080p", &list, &movie_as("The Matrix (1999)"), &HashMap::new());
        assert_eq!(p.kind, Some(MediaKind::Movie));
        assert_eq!(p.guessed_movie.as_deref(), Some("The Matrix (1999)"));
        assert_eq!(p.top.as_deref(), Some("The Matrix (1999)"));
        assert_eq!(p.own_folder, None, "it has a folder already");
        assert_eq!(to(&p), [Some("The Matrix (1999).mkv"), Some("The Matrix (1999).English.srt"), None, None, None]);
        assert_eq!(p.files[2].note.as_deref(), Some("An extra"));

        let tagged = NameOptions {
            movie: Some(MovieName { title: "The Matrix (1999)".into(), tmdb: Some(603), tag: true, own_folder: true }),
            ..NameOptions::default()
        };
        let p = plan("The.Matrix.1999.1080p", &list, &tagged, &HashMap::new());
        assert_eq!(p.top.as_deref(), Some("The Matrix (1999) {tmdb-603}"));
        assert_eq!(p.files[0].to.as_deref(), Some("The Matrix (1999).mkv"));
    }

    #[test]
    fn a_movie_in_parts() {
        let list = vec![
            TorrentFile { path: "Film/Film.CD1.avi".into(), size: 700 },
            TorrentFile { path: "Film/Film.CD2.avi".into(), size: 690 },
        ];
        let p = plan("Film", &list, &movie_as("Film (1980)"), &HashMap::new());
        assert_eq!(to(&p), [Some("Film (1980) - pt1.avi"), Some("Film (1980) - pt2.avi")]);
    }

    #[test]
    fn a_movie_thats_one_file() {
        let list = files(&["Heat.1995.1080p.mkv"]);
        let p = plan("Heat.1995.1080p.mkv", &list, &movie_as("Heat (1995)"), &HashMap::new());
        assert_eq!(p.top.as_deref(), Some("Heat (1995).mkv"));
        assert_eq!(p.own_folder.as_deref(), Some("Heat (1995)"));
    }
}
