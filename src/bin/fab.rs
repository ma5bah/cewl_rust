//! FAB — Files Already Bagged.
//!
//! Rust port of Robin Wood's fab.rb.
//! Copyright (c) Robin Wood (robin@digi.ninja); Licence: GPL

use cewl::cli::FabArgs;
use cewl::metadata;
use clap::{error::ErrorKind, Parser};

fn main() {
    let args = match FabArgs::try_parse() {
        Ok(a) => a,
        Err(e) => {
            let code = match e.kind() {
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => 0,
                _ => 1,
            };
            e.print().expect("clap error print");
            std::process::exit(code);
        }
    };

    let mut meta_data: Vec<String> = Vec::new();

    for path in &args.files {
        if let Some(data) = metadata::process_file(path, args.verbose) {
            meta_data.extend(data);
        }
    }

    // Mirror fab.rb: drop empty, dedupe, sort, print
    meta_data.retain(|x| !x.trim().is_empty());
    meta_data.sort();
    meta_data.dedup();

    if meta_data.is_empty() {
        println!("No data found");
    } else {
        println!("{}", meta_data.join("\n"));
    }
}
