//! Extract added/last modified times from git history and add/update copyright note.

pub mod config;
pub mod error;
pub mod file_ops;
pub mod git_ops;
pub mod regex_ops;

pub use config::Config;
pub use error::CError;
use file_ops::read_write_copyright;
use futures::future::join_all;
use futures::FutureExt;
use git_ops::get_added_mod_times_for_file;
use git_ops::get_files_on_ref;
use regex_ops::CopyrightCache;
use regex_ops::{generate_base_regex, generate_copyright_line};
use serde::Deserialize;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::path::Path;

#[derive(Debug, Deserialize, Hash, PartialEq)]
#[serde(untagged)]
pub enum CommentSign {
    LeftOnly(String),
    Enclosing(String, String),
}

pub async fn check_repo_copyright(repo_path_: &str, name: &str) -> Result<(), CError> {
    let config = Config::global();
    let repo_path = Path::new(repo_path_);
    let files_to_check = get_files_on_ref(repo_path_, "HEAD").await?;
    let files_to_check: Vec<&String> = config
        .filter_files(files_to_check.iter())
        .into_iter()
        .filter(|f| repo_path.join(Path::new(f)).is_file())
        .collect();

    println!("Checking {} files", files_to_check.len());

    let base_regex = generate_base_regex(name);
    let regex_cache = CopyrightCache::new(&base_regex);

    let check_and_fix_futures: Vec<_> = files_to_check
        .iter()
        .map(|filepath| {
            let comment_sign = config
                .get_comment_sign(filepath)
                .expect(&format!("Could not get comment sign for {}", filepath));
            let years_fut = get_added_mod_times_for_file(filepath, repo_path_).shared();
            let copyright_line_fut = generate_copyright_line(name, comment_sign, years_fut.clone());
            let filepath = repo_path.join(filepath);
            read_write_copyright(
                filepath,
                regex_cache.get_regex(comment_sign).unwrap(),
                years_fut,
                copyright_line_fut,
            )
        })
        .collect();

    let results = join_all(check_and_fix_futures).await;
    let failed: Vec<_> = results.iter().filter(|res| res.is_err()).collect();
    failed.iter().for_each(|res_err| {
        println!("Error: {}", res_err.as_ref().unwrap_err());
    });

    if !failed.is_empty() {
        return Err(CError::FixError);
    }

    Ok(())
}

pub fn get_hash<T: std::hash::Hash>(obj: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    obj.hash(&mut hasher);
    hasher.finish()
}
