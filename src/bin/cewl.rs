//! CeWL — Custom Word List generator.
//!
//! Rust port of Robin Wood's CeWL.
//! Copyright (c) Robin Wood (robin@digi.ninja); Licence: CC-BY-SA 2.0 / GPL-3+

use cewl::cli::{CewlArgs, VERSION};
use cewl::crawler;
use clap::{error::ErrorKind, Parser};
use std::fs::File;
use std::io::{BufWriter, Write};

fn open_output(path: Option<&std::path::PathBuf>) -> anyhow::Result<Box<dyn Write>> {
    match path {
        Some(p) => Ok(Box::new(BufWriter::new(File::create(p).map_err(|e| {
            anyhow::anyhow!("Couldn't open output file for writing: {e}")
        })?))),
        None => Ok(Box::new(std::io::stdout())),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    eprintln!("CeWL {VERSION} (Rust port of Robin Wood's CeWL https://digi.ninja/)");

    let args = match CewlArgs::try_parse() {
        Ok(mut a) => {
            if a.no_render {
                a.render = false;
            }
            if a.capture_url_structure {
                a.capture_paths = true;
                a.capture_subdomains = true;
                a.capture_domain = true;
            }
            if a.browser_user_data_dir.is_some() {
                a.headed = true;
            }
            a
        }
        Err(e) => {
            let code = match e.kind() {
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => 0,
                _ => 1,
            };
            e.print()?;
            std::process::exit(code);
        }
    };

    if let Err(e) = args.validate() {
        eprintln!("\n{e}\n");
        std::process::exit(1);
    }

    // Pre-open output files before crawling (Ruby aborts with code 2 on bad path)
    let mut word_out = match open_output(args.write.as_ref()) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    let mut email_out: Box<dyn Write> = if let Some(ref ef) = args.email_file {
        match open_output(Some(ef)) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(2);
            }
        }
    } else {
        open_output(None)?
    };
    let mut meta_out: Box<dyn Write> = if let Some(ref mf) = args.meta_file {
        match open_output(Some(mf)) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(2);
            }
        }
    } else {
        open_output(None)?
    };

    if args.verbose {
        eprintln!("Starting at {}", args.url);
        if args.render {
            if let Some(ref udd) = args.browser_user_data_dir {
                eprintln!("Browser: headed Chrome, profile dir: {}", udd.display());
                if let Some(ref pd) = args.browser_profile_dir {
                    eprintln!("  Profile: {pd}");
                }
            } else {
                eprintln!("Browser: headless Chromium");
            }
            if args.human_in_loop {
                eprintln!("Human-in-loop mode ON (timeout {}s)", args.human_timeout);
            }
        } else {
            eprintln!("Mode: static HTTP (--no-render)");
        }
    }

    let result = crawler::run(&args).await?;

    // --- Words ---
    if args.wordlist_enabled() {
        if args.verbose {
            if args.write.is_none() {
                eprintln!("Words found");
            } else {
                eprintln!("Writing words to file");
            }
        }
        let mut sorted: Vec<(&String, &u64)> = result.words.iter().collect();
        sorted.sort_by_key(|(_, &c)| std::cmp::Reverse(c));
        for (word, count) in sorted {
            if args.count {
                writeln!(word_out, "{word}, {count}")?;
            } else {
                writeln!(word_out, "{word}")?;
            }
        }
    }

    // --- Groups ---
    if args.groups > 0 {
        if args.verbose {
            if args.write.is_none() {
                eprintln!("Groups of words found");
            } else {
                eprintln!("Writing groups of words to file");
            }
        }
        let mut sorted: Vec<(&String, &u64)> = result.groups.iter().collect();
        sorted.sort_by_key(|(_, &c)| std::cmp::Reverse(c));
        for (phrase, count) in sorted {
            if args.count {
                writeln!(word_out, "{phrase}, {count}")?;
            } else {
                writeln!(word_out, "{phrase}")?;
            }
        }
    }

    // --- Emails ---
    if args.email {
        if result.emails.is_empty() {
            if args.verbose {
                eprintln!("No email addresses found");
            }
        } else {
            if args.verbose {
                eprintln!("Dumping email addresses to file");
            }
            if args.email_file.is_none() && args.wordlist_enabled() {
                writeln!(email_out)?;
            }
            if args.email_file.is_none() {
                writeln!(email_out, "Email addresses found")?;
                writeln!(email_out, "---------------------")?;
            }
            writeln!(email_out, "{}", result.emails.join("\n"))?;
        }
    }

    // --- Metadata / usernames ---
    if args.meta {
        if result.usernames.is_empty() {
            if args.verbose {
                eprintln!("No meta data found");
            }
        } else {
            if args.verbose {
                eprintln!("Dumping meta data to file");
            }
            if args.meta_file.is_none() && (args.email || args.wordlist_enabled()) {
                writeln!(meta_out)?;
            }
            if args.meta_file.is_none() {
                writeln!(meta_out, "Meta data found")?;
                writeln!(meta_out, "---------------")?;
            }
            writeln!(meta_out, "{}", result.usernames.join("\n"))?;
        }
    }

    Ok(())
}
