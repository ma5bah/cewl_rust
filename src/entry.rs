//! CLI dispatch: default URL spider (`cewl …`) vs `cewl fab …` vs standalone `fab` shim.

use crate::cli::{finalize_spider_in_place, CewlArgs, FabArgs, VERSION};
use crate::crawler;
use crate::metadata;
use anyhow::Result;
use clap::error::ErrorKind;
use std::ffi::OsString;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

/// Parse argv: `[exe, …]` — if second token is `fab`, parse as FAB; else spider.
pub fn parse_root(args: Vec<OsString>) -> Result<RootCmd, clap::Error> {
    let is_fab = args.len() > 1 && args.get(1).map(|a| a.as_encoded_bytes()) == Some(b"fab");
    if is_fab {
        let stitched: Vec<OsString> = std::iter::once(args[0].clone())
            .chain(args.into_iter().skip(2))
            .collect();
        FabArgs::try_parse_from(stitched).map(RootCmd::Fab)
    } else {
        CewlArgs::try_parse_from(args).map(RootCmd::Spider)
    }
}

pub enum RootCmd {
    Spider(CewlArgs),
    Fab(FabArgs),
}

pub fn exit_on_clap_err(e: clap::Error) -> ! {
    let code = match e.kind() {
        ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => 0,
        _ => 1,
    };
    e.print().expect("clap stderr");
    std::process::exit(code);
}

fn open_output(path: Option<&PathBuf>) -> Result<Box<dyn Write>> {
    match path {
        Some(p) => Ok(Box::new(BufWriter::new(File::create(p).map_err(|e| {
            anyhow::anyhow!("Couldn't open output file for writing: {e}")
        })?))),
        None => Ok(Box::new(std::io::stdout())),
    }
}

pub async fn run_spider(mut args: CewlArgs) -> Result<()> {
    finalize_spider_in_place(&mut args);
    if let Err(e) = args.validate() {
        eprintln!("\n{e}\n");
        std::process::exit(1);
    }

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

pub fn run_fab(args: FabArgs) {
    let mut meta_data: Vec<String> = Vec::new();
    for path in &args.files {
        if let Some(data) = metadata::process_file(path, args.verbose) {
            meta_data.extend(data);
        }
    }
    meta_data.retain(|x| !x.trim().is_empty());
    meta_data.sort();
    meta_data.dedup();

    if meta_data.is_empty() {
        println!("No data found");
    } else {
        println!("{}", meta_data.join("\n"));
    }
}

pub fn print_cewl_banner() {
    eprintln!("CeWL {VERSION} (Rust port of Robin Wood's CeWL https://digi.ninja/)");
}
