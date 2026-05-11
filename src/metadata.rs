//! Document metadata extraction — port of `cewl_lib.rb`.
//!
//! Copyright (c) Robin Wood (robin@digi.ninja); Licence: GPL

use regex::bytes::Regex as BytesRegex;
use roxmltree::Document;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::Command;
use std::sync::LazyLock;

// ----- byte-level PDF regexes (Ruby uses line-by-line ISO-8859-1 scan) -----
static RE_PDF_AUTHOR: LazyLock<BytesRegex> =
    LazyLock::new(|| BytesRegex::new(r"pdf:Author='([^']*)'").unwrap());
static RE_XAP_AUTHOR: LazyLock<BytesRegex> =
    LazyLock::new(|| BytesRegex::new(r"(?i)xap:Author='([^']*)'").unwrap());
static RE_DC_CREATOR: LazyLock<BytesRegex> =
    LazyLock::new(|| BytesRegex::new(r"(?i)dc:creator='([^']*)'").unwrap());
static RE_SLASH_AUTHOR: LazyLock<BytesRegex> =
    LazyLock::new(|| BytesRegex::new(r"(?i)/Author ?\(([^)]*)\)").unwrap());
static RE_XAP_CREATOR_TAG: LazyLock<BytesRegex> =
    LazyLock::new(|| BytesRegex::new(r"(?i)<xap:creator>(.*)</xap:creator>").unwrap());
static RE_XAP_AUTHOR_TAG: LazyLock<BytesRegex> =
    LazyLock::new(|| BytesRegex::new(r"(?i)<xap:Author>(.*)</xap:Author>").unwrap());
static RE_PDF_AUTHOR_TAG: LazyLock<BytesRegex> =
    LazyLock::new(|| BytesRegex::new(r"(?i)<pdf:Author>(.*)</pdf:Author>").unwrap());
static RE_DC_CREATOR_TAG: LazyLock<BytesRegex> =
    LazyLock::new(|| BytesRegex::new(r"(?i)<dc:creator>(.*)</dc:creator>").unwrap());

static RE_OFFICE_EXT: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?i)\.(doc|dot|ppt|pot|xls|xlt|pps)[xm]$|\.ppam$|\.xlsb$|\.xlam$").unwrap()
});
static RE_IGNORE_EXT: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?i)\.(php|aspx|cfm|asp|html|htm)$").unwrap());
static EXIF_LINE_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^\s*[^:]+:\s*(.+)$").unwrap());

fn push_nonempty(out: &mut Vec<String>, s: &str) {
    let t = s.trim();
    if !t.is_empty() {
        out.push(t.to_string());
    }
}

pub fn get_pdf_data(path: &Path, verbose: bool) -> Vec<String> {
    let mut meta = Vec::new();
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) => {
            if verbose {
                eprintln!("pdf open: {e}");
            }
            return meta;
        }
    };
    let reader = BufReader::new(file);
    for line_res in reader.split(b'\n') {
        let Ok(line) = line_res else { continue };
        for re in [
            &*RE_PDF_AUTHOR,
            &*RE_XAP_AUTHOR,
            &*RE_DC_CREATOR,
            &*RE_SLASH_AUTHOR,
            &*RE_XAP_CREATOR_TAG,
            &*RE_XAP_AUTHOR_TAG,
            &*RE_PDF_AUTHOR_TAG,
            &*RE_DC_CREATOR_TAG,
        ] {
            if let Some(cap) = re.captures(&line).and_then(|c| c.get(1)) {
                if let Ok(s) = std::str::from_utf8(cap.as_bytes()) {
                    push_nonempty(&mut meta, s);
                }
            }
        }
    }
    meta
}

pub fn get_doc_data(path: &Path, verbose: bool) -> Vec<String> {
    let mut data = Vec::new();
    let out = Command::new("exiftool")
        .args(["-Author", "-LastSavedBy", "-Creator"])
        .arg(path)
        .output();
    let Ok(output) = out else {
        if verbose {
            eprintln!("exiftool not found");
        }
        return data;
    };
    if !output.status.success() {
        if verbose {
            eprintln!("exiftool exit {}", output.status);
        }
        return data;
    }
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if let Some(cap) = EXIF_LINE_RE.captures(line) {
            if let Some(m) = cap.get(1) {
                push_nonempty(&mut data, m.as_str());
            }
        }
    }
    data
}

pub fn get_docx_data(path: &Path, verbose: bool) -> Vec<String> {
    let mut meta = Vec::new();
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) => {
            if verbose {
                eprintln!("docx open: {e}");
            }
            return meta;
        }
    };
    let mut archive = match zip::ZipArchive::new(file) {
        Ok(a) => a,
        Err(e) => {
            if verbose {
                eprintln!("zip: {e}");
            }
            return meta;
        }
    };
    let xml = match archive.by_name("docProps/core.xml") {
        Ok(mut z) => {
            use std::io::Read;
            let mut v = Vec::new();
            if z.read_to_end(&mut v).is_err() {
                return meta;
            }
            v
        }
        Err(_) => return meta,
    };
    let Ok(text) = std::str::from_utf8(&xml) else {
        return meta;
    };
    let Ok(doc) = Document::parse(text) else {
        return meta;
    };
    for n in doc.descendants() {
        let name = n.tag_name().name();
        if name == "creator" || name == "lastModifiedBy" {
            if let Some(t) = n.text() {
                push_nonempty(&mut meta, t);
            }
        }
    }
    meta
}

/// Ruby `process_file` — `None` means file missing or type unknown.
pub fn process_file(filename: &Path, verbose: bool) -> Option<Vec<String>> {
    if verbose {
        eprintln!("processing file: {}", filename.display());
    }
    if !filename.is_file() {
        return None;
    }

    let guessed = mime_guess::from_path(filename);
    let mime = guessed.first_raw().unwrap_or("");
    if mime.is_empty() {
        if verbose {
            eprintln!("Empty mime type");
        }
        return None;
    }
    if verbose {
        eprintln!("Checking {}", filename.display());
        eprintln!("  Mime type={mime}");
    }

    let name = filename.to_string_lossy();
    let meta: Option<Vec<String>>;

    if mime == "application/msword"
        || mime == "application/vnd.ms-excel"
        || mime == "application/vnd.ms-powerpoint"
    {
        if verbose {
            eprintln!("  Mime type says original office document");
        }
        meta = Some(get_doc_data(filename, verbose));
    } else if mime == "application/pdf" {
        if verbose {
            eprintln!("  Mime type says PDF");
        }
        let mut m = get_doc_data(filename, verbose);
        m.extend(get_pdf_data(filename, verbose));
        meta = Some(m);
    } else if RE_OFFICE_EXT.is_match(&name) {
        if verbose {
            eprintln!("  File extension says 2007 style office document");
        }
        meta = Some(get_docx_data(filename, verbose));
    } else if RE_IGNORE_EXT.is_match(&name) {
        if verbose {
            eprintln!("  Language file, can ignore");
        }
        meta = Some(vec![]);
    } else {
        if verbose {
            eprintln!("  Unknown file type");
        }
        meta = None;
    }

    if let Some(ref m) = meta {
        if verbose && !m.is_empty() {
            eprintln!("  Found {}", m.join(", "));
        }
    }
    meta
}
