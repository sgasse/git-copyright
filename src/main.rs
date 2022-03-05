use clap::Parser;
use copyright_git::{check_repo_copyright, Config};
use env_logger::TimestampPrecision;

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    /// Path to repository to check
    #[clap(short, long, default_value = "./")]
    repo: String,

    /// Name in copyright
    #[clap(short, long)]
    name: String,

    /// TOML file with config to use
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

    check_repo_copyright(&args.repo, &args.name, &config).await;
}
