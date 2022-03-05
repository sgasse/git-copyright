//! Extract added/last modified times from git history and add/update copyright note.

pub mod config;
pub mod file_ops;
pub mod git_ops;
pub mod regex_ops;

use chrono::DateTime;
use chrono::Datelike;
use chrono::Utc;
pub use config::Config;
use file_ops::check_and_fix_file;
use futures::future::join_all;
use git2::Repository;
use git_ops::{get_add_mod_ranges, get_files_on_ref};
use regex_ops::CopyrightCache;
use regex_ops::{generate_base_regex, generate_copyright_line};
use serde::Deserialize;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::Hasher;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Hash, PartialEq)]
#[serde(untagged)]
pub enum CommentSign {
    LeftOnly(String),
    Enclosing(String, String),
}

#[derive(Debug)]
pub struct AddedModifiedInfo {
    pub added: Option<DateTime<Utc>>,
    pub last_modified: Option<DateTime<Utc>>,
    pub original_path: PathBuf,
}

pub async fn check_repo_copyright(repo_path: &str, name: &str, config: &Config) {
    let repo_path = Path::new(repo_path);
    let repo = Repository::open(repo_path).expect("Could not open repository");
    let files_to_check = get_files_on_ref(&repo, "HEAD").expect("Could not get files on `HEAD`");
    let files_to_check = config.filter_files(files_to_check.iter());

    let mut add_mod_ranges = get_add_mod_ranges(&repo, files_to_check.iter().map(|x| *x)).unwrap();
    let add_mod_map: HashMap<u64, AddedModifiedInfo> = add_mod_ranges
        .drain(..)
        .map(|am_info| (get_hash(&am_info.original_path.to_str().unwrap()), am_info))
        .collect();

    let base_regex = generate_base_regex(name);
    let regex_cache = CopyrightCache::new(&base_regex);

    let check_and_fix_futures: Vec<_> = files_to_check
        .iter()
        .map(|filepath| {
            let years = get_years_info(filepath, &add_mod_map);
            let comment_sign = config
                .get_comment_sign(filepath)
                .expect(&format!("Could not get comment sign for {}", filepath));
            let copyright_line = generate_copyright_line(name, comment_sign, &years);
            let filepath = repo_path.join(filepath);
            check_and_fix_file(
                filepath,
                regex_cache.get_regex(comment_sign),
                years,
                copyright_line,
            )
        })
        .collect();

    join_all(check_and_fix_futures).await;
}

pub fn get_hash<T: std::hash::Hash>(obj: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    obj.hash(&mut hasher);
    hasher.finish()
}

pub fn get_years_info(filepath: &str, add_mod_map: &HashMap<u64, AddedModifiedInfo>) -> String {
    match add_mod_map.get(&get_hash(&filepath)) {
        None => "UNKNOWN".to_owned(),
        Some(am_info) => {
            if am_info.added.is_none() || am_info.last_modified.is_none() {
                "UNKNOWN".to_owned()
            } else {
                let added = am_info.added.unwrap().year().to_string();
                let modified = am_info.last_modified.unwrap().year().to_string();
                if added != modified {
                    [added, modified].join("-").to_owned()
                } else {
                    added
                }
            }
        }
    }
}
