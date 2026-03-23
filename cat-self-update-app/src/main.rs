use cat_self_update_lib::self_update;
use clap::{Parser, Subcommand};

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
    }
}
