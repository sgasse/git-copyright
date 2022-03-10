//! Extract added/last modified times from git history and add/update copyright note.

pub mod config;
pub mod file_ops;
pub mod git_ops;
pub mod regex_ops;

pub use config::Config;
use file_ops::check_and_fix_file;
use futures::future::join_all;
use git_ops::{get_add_mod_ranges, get_files_on_ref};
use regex_ops::CopyrightCache;
use regex_ops::{generate_base_regex, generate_copyright_line};
use serde::Deserialize;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::Hasher;
use std::path::{Path, PathBuf};

use thiserror::Error;

/// CheckCopyrightError enumerates possible errors returned by this library.
#[derive(Error, Debug)]
pub enum CheckCopyrightError {
    #[error("No comment sign found for file/extension")]
    NoCommentSign,

    #[error(transparent)]
    IOError(#[from] std::io::Error),
}

#[derive(Debug, Deserialize, Hash, PartialEq)]
#[serde(untagged)]
pub enum CommentSign {
    LeftOnly(String),
    Enclosing(String, String),
}

#[derive(Debug)]
pub struct AddedModifiedYears {
    pub original_path: PathBuf,
    pub years: String,
}

pub async fn check_repo_copyright(repo_path_: &str, name: &str, config: &Config) {
    let repo_path = Path::new(repo_path_);
    let files_to_check = get_files_on_ref(repo_path_, "HEAD")
        .await
        .expect("Could not get files on `HEAD`");
    let files_to_check: Vec<&String> = config
        .filter_files(files_to_check.iter())
        .into_iter()
        .filter(|f| repo_path.join(Path::new(f)).is_file())
        .collect();

    println!("Checking {} files", files_to_check.len());

    let mut add_mod_years = get_add_mod_ranges(files_to_check.iter().map(|x| *x), repo_path_)
        .await
        .unwrap();
    let add_mod_map: HashMap<u64, AddedModifiedYears> = add_mod_years
        .drain(..)
        .map(|am_info| (get_hash(&am_info.original_path.to_str().unwrap()), am_info))
        .collect();

    let base_regex = generate_base_regex(name);
    let regex_cache = CopyrightCache::new(&base_regex);

    let check_and_fix_futures: Vec<_> = files_to_check
        .iter()
        .map(|filepath| {
            let years = &add_mod_map.get(&get_hash(filepath)).unwrap().years;
            let comment_sign = config
                .get_comment_sign(filepath)
                .expect(&format!("Could not get comment sign for {}", filepath));
            let copyright_line = generate_copyright_line(name, comment_sign, years);
            let filepath = repo_path.join(filepath);
            check_and_fix_file(
                filepath,
                regex_cache.get_regex(comment_sign),
                years.clone(),
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
