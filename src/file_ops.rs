//! Check and update copyright of file.

use crate::CError;
use futures::join;
use futures::Future;
use regex::Regex;
use std::io::{BufRead, BufReader};
use std::sync::Arc;
use std::{path::Path, path::PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub async fn read_write_copyright(
    filepath: PathBuf,
    regex: Arc<Regex>,
    years_fut: impl Future<Output = String>,
    copyright_line: impl Future<Output = String>,
) -> Result<(), CError> {
    let (years, copyright_line) = join!(years_fut, copyright_line);

    // This could be re-written to read the file asynchronously until EOF or the first n
    // newlines are found.
    let file = std::fs::File::open(&filepath)
        .map_err(|_| CError::ReadError(filepath.display().to_string()))?;
    let file_header = BufReader::new(file).lines().take(3);

    for (line_nr, line_) in file_header.enumerate() {
        if let Ok(line_) = line_ {
            if let Some(cap) = regex.captures_iter(&line_).take(1).next() {
                if years == &cap[1] {
                    log::debug!(
                        "File {} has correct copyright with years {}",
                        filepath.display(),
                        years
                    );
                    return Ok(());
                } else {
                    println!(
                        "File {} has copyright with year(s) {} on line {} but should have {}",
                        filepath.display(),
                        &cap[1],
                        line_nr,
                        years
                    );
                    return write_copyright(&filepath, &copyright_line, Some(line_nr)).await;
                }
            }
        }
    }

    println!(
        "File {} has no copyright but should have {}",
        filepath.display(),
        years
    );
    write_copyright(&filepath, &copyright_line, None).await
}

async fn write_copyright(
    filepath: &Path,
    copyright_line: &str,
    line_nr: Option<usize>,
) -> Result<(), CError> {
    let mut file = tokio::fs::File::open(filepath)
        .await
        .map_err(|_| CError::ReadError(filepath.display().to_string()))?;
    let mut data = Vec::new();
    file.read_to_end(&mut data).await?;
    let mut data: Vec<&str> = std::str::from_utf8(&data)?.split("\n").collect();

    match line_nr {
        Some(line_nr) => {
            data[line_nr] = &copyright_line;
        }
        None => {
            data.insert(0, copyright_line);
        }
    }

    let mut file = tokio::fs::File::create(filepath)
        .await
        .map_err(|_| CError::WriteError(filepath.display().to_string()))?;
    file.write_all(data.join("\n").as_bytes())
        .await
        .map_err(|_| CError::WriteError(filepath.display().to_string()))?;

    Ok(())
}
