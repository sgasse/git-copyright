//! Extract added/modified times from git history.
//!

use crate::CError;
use chrono::Utc;
use tokio::process::Command;

pub async fn get_files_on_ref(repo_path: &str, ref_name: &str) -> Result<Vec<String>, CError> {
    let output = Command::new("git")
        .arg("ls-tree")
        .arg("-r")
        .arg(ref_name)
        .arg("--name-only")
        .current_dir(repo_path)
        .output();

    let output = output.await?;
    if !output.status.success() {
        return Err(CError::GitCmdError(
            String::from_utf8(output.stderr).map_err(|e| e.utf8_error())?,
        ));
    }

    Ok(parse_cmd_output(&output)?)
}

pub async fn get_added_mod_times_for_file(filepath: &str, cwd: &str) -> String {
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

    match commit_years.len() {
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
    }
}

pub async fn check_for_changes(repo_path: &str, fail_on_diff: bool) -> Result<(), CError> {
    let diff_files = get_diffs(repo_path).await?;
    if diff_files.len() > 0 {
        println!("Files changed:");
        for filepath in diff_files.iter() {
            println!("{}", filepath);
        }

        if fail_on_diff {
            return Err(CError::FilesChanged);
        }
    }

    Ok(())
}

async fn get_diffs<'a>(repo_path: &str) -> Result<Vec<String>, CError> {
    let output = Command::new("git")
        .arg("diff")
        .arg("--name-only")
        .current_dir(repo_path)
        .output();

    let output = output.await?;
    if !output.status.success() {
        return Err(CError::GitCmdError(
            String::from_utf8(output.stderr).map_err(|e| e.utf8_error())?,
        ));
    }

    Ok(parse_cmd_output(&output)?)
}

fn parse_cmd_output(output: &std::process::Output) -> Result<Vec<String>, CError> {
    let output = std::str::from_utf8(&output.stdout)?;
    let lines: Vec<String> = output
        .split('\n')
        .filter_map(|s| {
            let s = s.to_owned();
            match s.len() {
                0 => None,
                _ => Some(s),
            }
        })
        .collect();
    Ok(lines)
}
