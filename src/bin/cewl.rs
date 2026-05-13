//! CeWL — Custom Word List generator.

use cewl::entry::{exit_on_clap_err, parse_root, print_cewl_banner, run_fab, run_spider, RootCmd};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let argv: Vec<_> = std::env::args_os().collect();
    match parse_root(argv).unwrap_or_else(|e| exit_on_clap_err(e)) {
        RootCmd::Spider(args) => {
            print_cewl_banner();
            run_spider(args).await?;
        }
        RootCmd::Fab(args) => run_fab(args),
    }
    Ok(())
}
