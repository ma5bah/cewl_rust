//! Word extraction and counts (Ruby word_hash / group_word_hash).

use indexmap::IndexMap;
use regex::Regex;
use std::sync::LazyLock;

static NON_ALNUM: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^[:alnum:]]+").unwrap());
static NON_ALPHA: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^[:alpha:]]+").unwrap());

pub fn normalize_words(
    text: &str,
    lowercase: bool,
    with_numbers: bool,
    convert_umlauts: bool,
) -> String {
    let mut s = if lowercase {
        text.to_lowercase()
    } else {
        text.to_string()
    };
    if convert_umlauts {
        s = s
            .replace('ä', "ae")
            .replace('ö', "oe")
            .replace('ü', "ue")
            .replace('ß', "ss")
            .replace('Ä', "Ae")
            .replace('Ö', "Oe")
            .replace('Ü', "Ue");
    }
    if with_numbers {
        NON_ALNUM.replace_all(&s, " ").to_string()
    } else {
        NON_ALPHA.replace_all(&s, " ").to_string()
    }
}

pub fn count_words(
    normalized: &str,
    min_len: usize,
    max_len: Option<usize>,
    word_hash: &mut IndexMap<String, u64>,
    groups: i32,
    group_hash: &mut IndexMap<String, u64>,
) {
    let mut group_words: Vec<String> = Vec::new();
    for word in normalized.split_whitespace() {
        if word.len() < min_len {
            continue;
        }
        if let Some(mx) = max_len {
            if word.len() > mx {
                continue;
            }
        }
        *word_hash.entry(word.to_string()).or_insert(0) += 1;
        if groups > 0 {
            group_words.push(word.to_string());
            if group_words.len() > groups as usize {
                group_words.remove(0);
            }
            if group_words.len() == groups as usize {
                let joined = group_words.join(" ");
                *group_hash.entry(joined).or_insert(0) += 1;
            }
        }
    }
}
