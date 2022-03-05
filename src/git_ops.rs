//! Extract added/modified times from git history.
//!
//! This is built on top of libgit2. Renames are followed.

use super::AddedModifiedInfo;
use chrono::DateTime;
use chrono::{TimeZone, Utc};
use git2::{Commit, Diff, DiffFile, Repository, TreeWalkMode, TreeWalkResult};
use git2::{DiffFindOptions, Error};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Get all files in repository on given `refname`.
pub fn get_files_on_ref(repo: &Repository, refname: &str) -> Result<Vec<String>, Error> {
    let ref_id = repo.refname_to_id(refname)?;
    let commit = repo.find_commit(ref_id)?;
    let tree = commit.tree()?;
    let mut files: Vec<String> = vec![];
    tree.walk(TreeWalkMode::PreOrder, |_, entry| {
        if let Some(filename) = entry.name() {
            files.push(filename.into());
        }
        TreeWalkResult::Ok
    })?;

    Ok(files)
}

pub fn get_add_mod_ranges<'a>(
    repo: &'a Repository,
    files_to_check: impl Iterator<Item = &'a String>,
) -> Result<Vec<AddedModifiedInfo>, Error> {
    let mut unclear_add_mod = init_unclear_add_mod(files_to_check);
    let mut add_mod_ranges: Vec<AddedModifiedInfo> = vec![];

    let mut revwalk = repo.revwalk()?;
    revwalk.set_sorting(git2::Sort::TIME)?;
    revwalk.push_head()?;

    // Tracking the last commit is necessary to handle files added in the first commit
    let mut last_commit: Option<Commit> = None;

    // Walk the commit tree until we have clarified the added/modified times
    // of all files to check
    while !unclear_add_mod.is_empty() {
        last_commit = match revwalk.next() {
            Some(Ok(commit_id)) => {
                let commit = repo.find_commit(commit_id)?;
                if commit.parent_count() == 1 {
                    let mut diff = diff_to_prev(&repo, &commit)?;

                    // Check for renames/copies
                    diff.find_similar(Some(DiffFindOptions::new().all(true)))?;

                    for delta in diff.deltas() {
                        let new_file = delta.new_file();
                        update_last_modified(&new_file, &commit, &mut unclear_add_mod)?;

                        let old_file = delta.old_file();
                        check_and_close_file_added(
                            &old_file,
                            &commit,
                            &mut unclear_add_mod,
                            &mut add_mod_ranges,
                        )?;

                        track_rename(&new_file, &old_file, &mut unclear_add_mod)?;
                    }
                }

                Some(commit)
            }
            Some(Err(_)) => {
                println!("Error finding a commit during revparse!");
                None
            }
            None => {
                // No parents - first commit reached
                let first_commit_ts = to_utc(last_commit.unwrap().time());
                for (_, mut add_mod_entry) in unclear_add_mod.drain() {
                    add_mod_entry.added = Some(first_commit_ts.clone());
                    add_mod_ranges.push(add_mod_entry);
                }
                None
            }
        };
    }
    Ok(add_mod_ranges)
}

fn check_and_close_file_added(
    old_file: &DiffFile,
    commit: &Commit,
    unclear_add_mod: &mut HashMap<PathBuf, AddedModifiedInfo>,
    add_mod_ranges: &mut Vec<AddedModifiedInfo>,
) -> Result<(), Error> {
    let file_path = old_file.path().unwrap();
    if let Some(add_mod_entry) = unclear_add_mod.get(file_path) {
        if !old_file.exists() {
            let added_ts = to_utc(commit.time());
            log::debug!("File {} was added on {:#?}", file_path.display(), added_ts);

            add_mod_ranges.push(AddedModifiedInfo {
                added: Some(added_ts),
                last_modified: add_mod_entry.last_modified,
                original_path: add_mod_entry.original_path.clone(), // TODO: Replace clone
            });

            unclear_add_mod.remove(file_path);
        }
    }
    Ok(())
}

fn track_rename(
    new_file: &DiffFile,
    old_file: &DiffFile,
    unclear_add_mod: &mut HashMap<PathBuf, AddedModifiedInfo>,
) -> Result<(), Error> {
    let new_file_path = new_file.path().unwrap();
    let old_file_path = old_file.path().unwrap();
    if new_file_path != old_file_path {
        // File was renamed - try to remove and reinsert with name before rename
        match unclear_add_mod.remove(new_file_path) {
            Some(add_mod_entry) => {
                unclear_add_mod.insert(old_file_path.to_path_buf(), add_mod_entry);
            }
            None => {
                log::warn!(
                    "Could not find {} to track rename to {}",
                    new_file_path.display(),
                    old_file_path.display()
                );
            }
        }
    }
    Ok(())
}

fn update_last_modified(
    new_file: &DiffFile,
    commit: &Commit,
    unclear_add_mod: &mut HashMap<PathBuf, AddedModifiedInfo>,
) -> Result<(), Error> {
    let file_path = new_file.path().unwrap();
    if let Some(add_mod_entry) = unclear_add_mod.get(file_path) {
        // Path is still not clarified
        let file_mod_time = to_utc(commit.time());
        match add_mod_entry.last_modified {
            Some(last_modified) if last_modified > file_mod_time => return Ok(()),
            _ => {
                let updated_entry = AddedModifiedInfo {
                    added: None,
                    last_modified: Some(file_mod_time),
                    original_path: add_mod_entry.original_path.clone(),
                };
                unclear_add_mod.insert(file_path.to_path_buf(), updated_entry);

                log::debug!(
                    "Updated last_modified of file {} to {:#?}",
                    file_path.display(),
                    file_mod_time
                );
            }
        }
    }

    Ok(())
}

fn init_unclear_add_mod<'a>(
    files: impl Iterator<Item = &'a String>,
) -> HashMap<PathBuf, AddedModifiedInfo> {
    files
        .map(|filename| {
            let file_path = Path::new(filename);
            (
                file_path.to_path_buf(),
                AddedModifiedInfo {
                    added: None,
                    last_modified: None,
                    original_path: file_path.to_path_buf(),
                },
            )
        })
        .collect::<HashMap<_, _>>()
}

fn diff_to_prev<'a>(repo: &'a Repository, commit: &Commit) -> Result<Diff<'a>, Error> {
    let tree = commit.tree()?;
    let prev_commit = commit.parent(0)?;
    let prev_tree = prev_commit.tree()?;

    Ok(repo.diff_tree_to_tree(Some(&prev_tree), Some(&tree), None)?)
}

fn to_utc(g_time: git2::Time) -> DateTime<Utc> {
    Utc.timestamp(g_time.seconds(), 0)
}
