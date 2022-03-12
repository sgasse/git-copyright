//! Parse configuration.
//!
//! If no custom configuration is specified, we fall back to the default
//! configuration which is included as bytes in the compiled binary.

use crate::CError;
use crate::CommentSign;
use glob::Pattern;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Config {
    comment_sign_map: HashMap<String, CommentSign>,
    ignore_files: Vec<String>,
    ignore_dirs: Vec<String>,
    #[serde(skip)]
    glob_pattern: Option<Vec<Pattern>>,
}

impl Config {
    pub fn default() -> Self {
        let cfg_bytes = include_bytes!("./default_cfg.yml");
        let cfg_str = String::from_utf8_lossy(cfg_bytes);
        Self::from_str(&cfg_str).expect("Failed to load default config")
    }

    pub fn from_file(cfg_file: &str) -> Result<Self, CError> {
        let cfg_str = std::fs::read_to_string(cfg_file)?;
        Self::from_str(&cfg_str)
    }

    pub fn from_str(cfg_str: &str) -> Result<Self, CError> {
        let mut cfg = serde_yaml::from_str::<Self>(&cfg_str)
            .map_err(|e| CError::ConfigError(format!("Could not deserialize config: {}", e)))?;
        cfg.build_glob_pattern();
        return Ok(cfg);
    }

    pub fn get_comment_sign(&self, filename: &str) -> Option<&CommentSign> {
        let filename = Path::new(filename);
        let ext_filename = match filename.extension() {
            Some(ext) => ext,
            None => filename.file_name().expect("Could not decode filename"),
        };
        let ext_filename = ext_filename
            .to_str()
            .expect("Could not interpret filename/extension");

        self.comment_sign_map.get(ext_filename)
    }

    pub fn filter_files<'a>(&self, files: impl Iterator<Item = &'a String>) -> Vec<&'a String> {
        if self.glob_pattern.is_none() {
            log::warn!("No glob patterns to ignore found");
        }

        files
            .filter_map(|filepath| {
                if let Some(patterns) = self.glob_pattern.as_ref() {
                    for pattern in patterns {
                        if pattern.matches(filepath) {
                            return None;
                        }
                    }
                }

                Some(filepath)
            })
            .collect()
    }

    fn build_glob_pattern(&mut self) {
        self.glob_pattern = Some(
            self.ignore_files
                .iter()
                .chain(self.ignore_dirs.iter())
                .filter_map(|expr| match Pattern::new(expr) {
                    Ok(pattern) => Some(pattern),
                    Err(_) => {
                        log::error!("Could not compile pattern {}", expr);
                        None
                    }
                })
                .collect(),
        );
    }
}

#[cfg(test)]
mod test {

    use super::{CommentSign, Config};

    #[test]
    fn test_config_from_file() {
        let cfg = Config::from_file("./src/default_cfg.yml").unwrap();
        assert_eq!(
            cfg.get_comment_sign("file.rs"),
            Some(&CommentSign::LeftOnly("//".into()))
        );

        let cfg = Config::default();
        assert_eq!(
            cfg.get_comment_sign("file.py"),
            Some(&CommentSign::LeftOnly("#".into()))
        );
    }

    #[test]
    fn test_filter_files() {
        let unfiltered: Vec<String> = vec!["dev/myfile.rs", "general/myfile.py", "another_file.py"]
            .iter()
            .map(|&elm| elm.into())
            .collect();
        let to_filter: Vec<String> = vec![
            "filter_me.txt",
            "./dev/I_want_out.txt",
            "dev/__pycache__/valid_file_in_ignored_folder.py",
            "dev/corner__pycache__case/myfile.py",
        ]
        .iter()
        .map(|&elm| elm.into())
        .collect();

        let cfg = Config::default();
        assert!(cfg.glob_pattern.is_some());

        let filtered_files = cfg.filter_files(unfiltered.iter().chain(to_filter.iter()));
        for filename in unfiltered.iter() {
            assert!(filtered_files.contains(&filename));
        }
        for filename in to_filter.iter() {
            assert!(!filtered_files.contains(&filename));
        }
    }
}
