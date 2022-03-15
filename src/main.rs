//! Add/update copyright notes according to history.

use anyhow::{Context, Result};
use clap::Parser;
use env_logger::TimestampPrecision;
use git_copyright::{check_repo_copyright, Config};
use std::time::Instant;

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    /// Path to repository to check
    #[clap(short, long, default_value = "./")]
    repo: String,

    /// Name in copyright
    #[clap(short, long)]
    name: String,

    /// YAML file with config to use
    #[clap(short, long, default_value = "")]
    config: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    env_logger::builder()
        .format_timestamp(Some(TimestampPrecision::Millis))
        .init();

    match args.config.as_str() {
        "" => {
            log::info!("Using default configuration");
            Config::default().assign();
        }
        cfg_file => {
            log::info!("Using config {}", cfg_file);
            Config::from_file(cfg_file)
                .context(format!("Unable to get config from file {}", cfg_file))?
                .assign();
        }
    }

    let start = Instant::now();
    check_repo_copyright(&args.repo, &args.name).await?;
    let duration_s = start.elapsed().as_millis() as f32 / 1000.0;
    println!("Copyrights checked and updated in {:0.3}s", duration_s);

    Ok(())
}
