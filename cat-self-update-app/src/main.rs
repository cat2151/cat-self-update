use cat_self_update_lib::{check_remote_commit, self_update};
use clap::{Parser, Subcommand};

const BUILD_COMMIT_HASH: &str = env!("BUILD_COMMIT_HASH");

#[derive(Parser)]
#[command(name = "cat-self-update")]
#[command(about = "Demo app that self-updates via cargo install")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Self-update the application from GitHub
    Update,
    /// Print the build-time commit hash
    Hash,
    /// Compare the build-time commit hash with the remote main branch
    Check,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Update => {
            if let Err(e) = self_update("cat2151", "cat-self-update", &[]) {
                eprintln!("Update failed: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Hash => println!("{}", BUILD_COMMIT_HASH),
        Commands::Check => match check_remote_commit(
            "cat2151",
            "cat-self-update",
            "main",
            BUILD_COMMIT_HASH,
        ) {
            Ok(result) => println!("{result}"),
            Err(e) => {
                eprintln!("Check failed: {}", e);
                std::process::exit(1);
            }
        },
    }
}
