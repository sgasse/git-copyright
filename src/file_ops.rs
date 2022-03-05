use regex::Regex;
use std::io::{BufRead, BufReader};
use std::sync::Arc;
use std::{path::Path, path::PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub async fn check_and_fix_file(
    filepath: PathBuf,
    regex: Arc<Regex>,
    years: String,
    copyright_line: String,
) {
    // This could be re-written to read the file asynchronously until EOF of the first n
    // newlines are found.
    let file = std::fs::File::open(&filepath)
        .expect(&format!("Could not open file {}", filepath.display()));
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
                    return;
                } else {
                    println!(
                        "File {} has copyright with year(s) {} on line {} but should have {} - fixed",
                        filepath.display(),
                        &cap[1],
                        line_nr,
                        years
                    );
                    write_copyright(&filepath, &copyright_line, Some(line_nr)).await;
                    return;
                }
            }
        }
    }

    write_copyright(&filepath, &copyright_line, None).await;
}

async fn write_copyright(filepath: &Path, copyright_line: &str, line_nr: Option<usize>) {
    let mut file = tokio::fs::File::open(filepath).await.expect(&format!(
        "Could not open file {} asynchronously",
        filepath.display()
    ));
    let mut data = Vec::new();
    file.read_to_end(&mut data)
        .await
        .expect("Could not read file");
    let mut data: Vec<&str> = std::str::from_utf8(&data)
        .expect("Could not decode file content to utf8")
        .split("\n")
        .collect();

    match line_nr {
        Some(line_nr) => {
            data[line_nr] = &copyright_line;
        }
        None => {
            data.insert(0, copyright_line);
        }
    }

    file.write_all(data.join("\n").as_bytes())
        .await
        .expect(&format!("Unable to write file {:?}", filepath));
}
