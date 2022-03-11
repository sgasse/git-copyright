//! Extract added/modified times from git history.
//!

use crate::AddedModifiedYears;
use crate::CheckCopyrightError;
use chrono::Utc;
use futures::future::join_all;
use std::path::Path;
use tokio::process::Command;

/// Get all files in repository on given `refname`.
pub async fn get_files_on_ref(
    repo_path: &str,
    ref_name: &str,
) -> Result<Vec<String>, CheckCopyrightError> {
    let output = Command::new("git")
        .arg("ls-tree")
        .arg("-r")
        .arg(ref_name)
        .arg("--name-only")
        .current_dir(repo_path)
        .output();

    let output = output.await?;

    // TODO: Handle error case

    let output = std::str::from_utf8(&output.stdout).expect("Could not decode command output");
    Ok(output
        .split('\n')
        .filter_map(|s| {
            let s = s.to_owned();
            match s.len() {
                0 => None,
                _ => Some(s),
            }
        })
        .collect())
}

pub async fn get_add_mod_ranges<'a>(
    files_to_check: impl Iterator<Item = &'a String>,
    repo_path: &str,
) -> Result<Vec<AddedModifiedYears>, CheckCopyrightError> {
    let time_futures: Vec<_> = files_to_check
        .map(|filepath| get_added_mod_times_for_file(filepath, repo_path))
        .collect();
    Ok(join_all(time_futures).await)
}

async fn get_added_mod_times_for_file(filepath: &str, cwd: &str) -> AddedModifiedYears {
    let output = Command::new("git")
        .arg("log")
        .arg("--follow")
        .arg("-m")
        .arg("--pretty=%ci")
        .arg(filepath)
        .current_dir(cwd)
        .output();
    let output = output.await.unwrap().stdout;
    let commit_years: Vec<String> = std::str::from_utf8(&output)
        .unwrap()
        .split('\n')
        .filter_map(|s| {
            // Take only first four chars (the year) from strings that are longer than zero
            let s = s.to_owned();
            match s.len() {
                0 => None,
                _ => Some(s.chars().take(4).collect()),
            }
        })
        .collect();

    let years_string = match commit_years.len() {
        0 => {
            log::debug!("File {} is untracked, add current year", filepath);
            Utc::now().date().format("%Y").to_string()
        }
        1 => {
            log::debug!("File {} was only committed once", filepath);
            commit_years[0].clone()
        }
        num_commits => {
            log::debug!("File {} was modified {} times", filepath, num_commits);
            let added = commit_years[commit_years.len() - 1].clone();
            let last_modified = commit_years[0].clone();
            match added == last_modified {
                true => added,
                false => format!("{}-{}", added, last_modified),
            }
        }
    };

    AddedModifiedYears {
        original_path: Path::new(filepath).to_path_buf(),
        years: years_string,
    }
}
