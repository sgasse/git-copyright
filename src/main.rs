//! Add/update copyright notes according to history.

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
async fn main() {
    let args = Args::parse();

    env_logger::builder()
        .format_timestamp(Some(TimestampPrecision::Millis))
        .init();

    let config = match args.config.as_str() {
        "" => {
            log::info!("Using default configuration");
            Config::default()
        }
        cfg_file => {
            log::info!("Using config {}", cfg_file);
            Config::from_file(cfg_file)
        }
    };

    let start = Instant::now();
    check_repo_copyright(&args.repo, &args.name, &config).await;
    let duration_s = start.elapsed().as_millis() as f32 / 1000.0;
    println!("Finished in {:0.3}s", duration_s);
}
