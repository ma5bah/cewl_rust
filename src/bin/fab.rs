//! FAB — thin wrapper: forwards to `cewl fab …` for scripts that still invoke `fab`.

use cewl::entry::{exit_on_clap_err, parse_root, run_fab, RootCmd};

fn main() -> anyhow::Result<()> {
    let mut argv: Vec<std::ffi::OsString> = std::env::args_os().collect();
    if !argv.is_empty() {
        argv.insert(1, "fab".into());
    }
    match parse_root(argv).unwrap_or_else(|e| exit_on_clap_err(e)) {
        RootCmd::Fab(args) => run_fab(args),
        RootCmd::Spider(_) => {
            eprintln!("internal error: fab shim expected fab sub-parse");
            std::process::exit(70);
        }
    }
    Ok(())
}
