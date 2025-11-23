use clap::{CommandFactory, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "tdd-cli", author = "xpepper", version, about = "Autonomous Multi-Agent TDD Machine", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Placeholder init command until real implementation lands
    Init,
    /// Placeholder run command until real implementation lands
    Run,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Init) => println!("init command not implemented yet"),
        Some(Commands::Run) => println!("run command not implemented yet"),
        None => Cli::command().print_help().expect("failed to print help"),
    }
}
