use clap::{Args, CommandFactory, Parser, Subcommand};

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
    Status,
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

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Init(args)) => handle_init(&args),
        Some(Commands::Run(args)) => handle_run(&args),
        Some(Commands::Step(args)) => handle_step(&args),
        Some(Commands::Status) => handle_status(),
        Some(Commands::Doctor) => handle_doctor(),
        None => Cli::command().print_help().expect("failed to print help"),
    }
}

fn handle_init(args: &InitArgs) {
    println!("init not implemented yet: using config {}", args.config);
}

fn handle_run(args: &RunArgs) {
    println!("run not implemented yet: steps={}, config={}", args.steps, args.config);
}

fn handle_step(args: &StepArgs) {
    println!("step not implemented yet: config {}", args.config);
}

fn handle_status() {
    println!("status not implemented yet");
}

fn handle_doctor() {
    println!("doctor not implemented yet");
}
