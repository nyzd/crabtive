use clap::Parser;
use core::{AccountChecker, Config};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    usernames: Vec<String>,

    #[arg(short, long)]
    config: PathBuf,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let config = Config::try_from(args.config.clone()).unwrap();
    let checker = AccountChecker::from(config.clone());

    for username in args.usernames {
        println!("Results for '{}'", username);
        let results = checker.check_accounts(&username).await.unwrap();

        for result in results {
            println!("[*] {result}");
        }

        println!("{}", "-".repeat(30));
    }
}
