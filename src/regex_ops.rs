//! Compile and cache copyright regexes.
//!
//! This module contains functions to parse existing copyright notes. Regexes
//! are compiled once per comment sign and stored in a cache.

use super::get_hash;
use super::CommentSign;
use regex::Regex;
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::sync::RwLock;

pub struct CopyrightCache {
    regexes: RwLock<HashMap<u64, Arc<Regex>>>,
    base_regex: String,
}

impl CopyrightCache {
    pub fn new(base_regex: &str) -> Self {
        CopyrightCache {
            regexes: RwLock::new(HashMap::new()),
            base_regex: base_regex.to_owned(),
        }
    }

    pub fn get_regex(&self, comment_sign: &CommentSign) -> Arc<Regex> {
        let c_sign_hash = get_hash(comment_sign);

        if let Some(regex) = self.regexes.read().unwrap().get(&c_sign_hash) {
            return Arc::clone(regex);
        }

        log::debug!("Initializing regex for comment sign {:?}", &comment_sign);
        let regex = Arc::new(generate_comment_regex(&self.base_regex, comment_sign).unwrap());
        self.regexes
            .write()
            .unwrap()
            .insert(get_hash(comment_sign), Arc::clone(&regex));
        regex
    }
}

pub fn generate_base_regex(name: &str) -> String {
    [
        r"Copyright \(c\)",
        &escape_for_regex(name),
        r"(\d{4}(-\d{4}){0,1})",
    ]
    .join(" ")
}

pub async fn generate_copyright_line(
    name: &str,
    comment_sign: &CommentSign,
    years_fut: impl Future<Output = String>,
) -> String {
    let years = years_fut.await;
    match comment_sign {
        CommentSign::LeftOnly(ref left) => [left, "Copyright (c)", name, &years].join(" "),
        CommentSign::Enclosing(ref left, ref right) => {
            [left, "Copyright (c)", name, &years, right].join(" ")
        }
    }
}

fn escape_for_regex(text: &str) -> String {
    text.chars()
        .map(|char| match char {
            '*' => String::from(r"\*"),
            '.' => String::from(r"\."),
            other => String::from(other),
        })
        .collect::<Vec<String>>()
        .as_slice()
        .join("")
}

fn generate_comment_regex(base_regex: &str, comment_sign: &CommentSign) -> Result<Regex, String> {
    let full_regex_str = match comment_sign {
        CommentSign::LeftOnly(left_sign) => {
            ["^", &escape_for_regex(&left_sign), " ", base_regex, "$"].join("")
        }
        CommentSign::Enclosing(left_sign, right_sign) => [
            "^",
            &escape_for_regex(&left_sign),
            " ",
            base_regex,
            " ",
            &escape_for_regex(&right_sign),
            "$",
        ]
        .join(""),
    };

    Ok(Regex::new(&full_regex_str).unwrap())
}

#[cfg(test)]
mod test {

    use super::escape_for_regex;
    use super::CommentSign;
    use super::{generate_base_regex, generate_comment_regex};
    use regex::Regex;

    #[test]
    fn test_generate_file_regex() {
        let file_header = "// Copyright (c) DummyCompany Ltd. 2020-2021";
        let regex = generate_comment_regex(
            &generate_base_regex("DummyCompany Ltd."),
            &CommentSign::LeftOnly("//".into()),
        )
        .unwrap();
        assert!(regex.is_match(file_header));
    }

    #[test]
    fn test_escape_for_regex() {
        assert_eq!(escape_for_regex("/"), r"/");
        assert_eq!(escape_for_regex("//"), r"//");
        assert_eq!(escape_for_regex("/*"), r"/\*");
        assert_eq!(escape_for_regex("*/"), r"\*/");
        assert_eq!(escape_for_regex("#"), "#");
    }

    #[test]
    fn test_rs_regex() {
        let header = "// Copyright (c) DummyCompany Ltd. 2022";
        let full_regex_str = r"^// Copyright \(c\) DummyCompany Ltd\. (\d{4}(-\d{4}){0,1})$";
        let regex = Regex::new(full_regex_str).unwrap();
        assert!(regex.is_match(header));
    }

    #[test]
    fn test_star_in_regex() {
        let file_header = "/* Copyright (c) DummyCompany Ltd. 2020-2021 */";
        let regex_str = r"^/\* Copyright \(c\) DummyCompany Ltd. \d{4}(-\d{4}){0,1} \*/$";
        let regex = Regex::new(regex_str).unwrap();
        assert!(regex.is_match(file_header));
    }

    #[test]
    fn test_forward_slash_in_regex() {
        let file_header = "// Copyright (c) DummyCompany Ltd. 2020-2021";
        let regex_str = r"^// Copyright \(c\) DummyCompany Ltd. \d{4}(-\d{4}){0,1}$";
        let regex = Regex::new(regex_str).unwrap();
        assert!(regex.is_match(file_header));
    }

    #[test]
    fn test_generate_base_regex() {
        let name = "DummyCompany Ltd.";
        let base_regex = generate_base_regex(name);
        assert_eq!(
            base_regex,
            r"Copyright \(c\) DummyCompany Ltd\. (\d{4}(-\d{4}){0,1})"
        );
    }

    #[test]
    fn test_regex_match() {
        let valid_copyrights = [
            "# Copyright (c) DummyCompany Ltd. 2019",
            "# Copyright (c) DummyCompany Ltd. 2020-2021",
        ];
        let invalid_copyrights = [
            "# Copyright (c) DummyCompany Ltd. 2019-",
            "# Copyright (c) DummyCompany Ltd. 2020-2021-2023",
            "# Copyright (c) DummyCompany Ltd. 20202021",
        ];

        let copyright_re_str = r"^# Copyright \(c\) DummyCompany Ltd. \d{4}(-\d{4}){0,1}$";
        let copyright_re = Regex::new(copyright_re_str).unwrap();

        for example in valid_copyrights {
            assert!(copyright_re.is_match(example));
        }

        for example in invalid_copyrights {
            assert!(!copyright_re.is_match(example));
        }
    }
}
