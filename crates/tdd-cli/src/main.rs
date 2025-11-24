use std::process;

use anyhow::Result;
use clap::{Args, CommandFactory, Parser, Subcommand};

use tdd_cli::{executor, init, status};

#[derive(Parser, Debug)]
#[command(name = "tdd-cli", author = "xpepper", version, about = "Autonomous Multi-Agent TDD Machine", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initialize (or update) a kata workspace for the autonomous TDD flow
    Init(InitArgs),
    /// Run N autonomous TDD steps (defaults to 1, requires initialized workspace)
    Run(RunArgs),
    /// Execute a single TDD step with the current role
    Step(StepArgs),
    /// Print the current automation status (role, step, last commit)
    Status(StatusArgs),
    /// Diagnose environment issues (tooling, config files, git status)
    Doctor,
}

#[derive(Args, Debug, Default)]
struct InitArgs {
    /// Optional path to the kata configuration file (defaults to ./tdd.yaml)
    #[arg(long, default_value = "tdd.yaml")]
    config: String,
}

#[derive(Args, Debug, Default)]
struct RunArgs {
    /// Number of TDD steps to execute
    #[arg(long, default_value_t = 1)]
    steps: u32,
    /// Optional path to configuration file
    #[arg(long, default_value = "tdd.yaml")]
    config: String,
}

#[derive(Args, Debug, Default)]
struct StepArgs {
    /// Optional path to configuration file
    #[arg(long, default_value = "tdd.yaml")]
    config: String,
}

#[derive(Args, Debug, Default)]
struct StatusArgs {
    /// Optional path to configuration file
    #[arg(long, default_value = "tdd.yaml")]
    config: String,
}

fn main() {
    if let Err(err) = run_cli() {
        eprintln!("Error: {err:?}");
        process::exit(1);
    }
}

fn run_cli() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Init(args)) => handle_init(&args),
        Some(Commands::Run(args)) => handle_run(&args),
        Some(Commands::Step(args)) => handle_step(&args),
        Some(Commands::Status(args)) => handle_status(&args),
        Some(Commands::Doctor) => handle_doctor(),
        None => {
            Cli::command().print_help()?;
            Ok(())
        }
    }
}

fn handle_init(args: &InitArgs) -> Result<()> {
    let result = init::initialize_workspace(&args.config)?;

    println!("\n🎉 Workspace initialized successfully!");
    if result.workspace_exists {
        println!("   Integrated with existing Rust project");
    }
    println!("\nNext steps:");
    println!("  1. Edit kata.md to describe your kata");
    println!("  2. Configure {} if needed", args.config);
    println!("  3. Set your API key: export OPENAI_API_KEY=your-key");
    println!("  4. Run: cargo run -p tdd-cli -- run --steps 3");

    Ok(())
}

fn handle_run(args: &RunArgs) -> Result<()> {
    let summary = executor::run_steps(&args.config, args.steps)?;
    report_summary("run", summary);
    Ok(())
}

fn handle_step(args: &StepArgs) -> Result<()> {
    let summary = executor::run_steps(&args.config, 1)?;
    report_summary("step", summary);
    Ok(())
}

fn handle_status(args: &StatusArgs) -> Result<()> {
    let report = status::gather_status(&args.config)?;
    for line in report.format_lines() {
        println!("{line}");
    }
    Ok(())
}

fn handle_doctor() -> Result<()> {
    println!("doctor not implemented yet");
    Ok(())
}

fn report_summary(command: &str, summary: executor::ExecutionSummary) {
    if summary.executed == summary.requested {
        println!(
            "{command} completed {} step(s) successfully.",
            summary.executed
        );
    } else {
        println!(
            "{command} executed {} of {} requested step(s) due to max_steps limit.",
            summary.executed, summary.requested
        );
    }
}
