#![allow(dead_code, unused_imports)]

pub mod adapter;
pub mod error;
pub mod runner;

use adapter::create_adapter;
use clap::Parser;
use kerrigan_storage::StorageEngine;
use runner::RunConfig;
use tracing::{debug, info, warn};

#[derive(Parser)]
#[command(
    name = "kerrigan-cli",
    about = "Kerrigan multi-agent orchestration CLI",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Create a new task and run it to completion
    Run {
        /// Task description
        #[arg(long, help = "What the task should accomplish")]
        summary: String,

        /// Working directory for the task
        #[arg(long, default_value = ".", help = "Working directory")]
        workdir: String,

        /// Agent adapter to use
        #[arg(long, default_value = "claude", help = "Agent adapter: claude or kimi")]
        adapter: String,

        /// Skills directory to inject into the agent
        #[arg(long, help = "Path to skills directory")]
        skills_dir: Option<String>,

        /// Max continue turns before rotating session
        #[arg(long, default_value = "10", help = "Rotate session after N turns")]
        max_turns: usize,

        /// Verbosity: -v for info, -vv for debug (includes agent output streaming)
        #[arg(short, long, action = clap::ArgAction::Count)]
        verbose: u8,
    },

    /// Resume an existing task
    Resume {
        /// Task ID to resume
        #[arg(help = "The task ID to resume")]
        task_id: String,

        /// Agent adapter to use
        #[arg(long, default_value = "claude", help = "Agent adapter: claude or kimi")]
        adapter: String,

        /// Skills directory to inject into the agent
        #[arg(long, help = "Path to skills directory")]
        skills_dir: Option<String>,

        /// Max continue turns before rotating session
        #[arg(long, default_value = "10", help = "Rotate session after N turns")]
        max_turns: usize,

        /// Verbosity: -v for info, -vv for debug (includes agent output streaming)
        #[arg(short, long, action = clap::ArgAction::Count)]
        verbose: u8,
    },

    /// Show status of tasks or a specific task's DAG
    Status {
        /// Task ID (omit to list all tasks)
        #[arg(help = "Task ID (omit to list all tasks)")]
        task_id: Option<String>,
    },

    /// Send a message to a running task
    Tell {
        /// Task ID
        #[arg(help = "The task ID to send the message to")]
        task_id: String,

        /// Message content
        #[arg(long, help = "Message to post to the task's board")]
        message: String,
    },
}

fn init_tracing(verbose: u8) {
    use tracing_subscriber::EnvFilter;

    let filter = match verbose {
        0 => "kerrigan_cli=warn",
        1 => "kerrigan_cli=info",
        _ => "kerrigan_cli=debug,kerrigan=debug",
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(filter)),
        )
        .with_target(false)
        .without_time()
        .init();
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli).await {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<(), error::CliError> {
    match cli.command {
        Command::Run {
            summary,
            workdir,
            adapter: adapter_name,
            skills_dir,
            max_turns,
            verbose,
        } => {
            init_tracing(verbose);

            let engine = StorageEngine::default_root().map_err(error::CliError::Storage)?;
            engine.init().map_err(error::CliError::Storage)?;

            let adapter = create_adapter(&adapter_name)
                .ok_or_else(|| error::CliError::UnknownAdapter(adapter_name.clone()))?;

            let task_id = engine
                .task_create(&summary, &workdir)
                .map_err(error::CliError::Storage)?;

            info!(task_id = %task_id, summary = %summary, "Created task");

            let skills_dir = skills_dir
                .map(std::path::PathBuf::from)
                .or_else(|| Some(engine.skills_dir()));

            let config = RunConfig {
                task_id: task_id.clone(),
                task_summary: summary,
                workdir: std::path::PathBuf::from(&workdir),
                skills_dir,
                shared_dir: Some(engine.shared_dir()),
                max_turns_per_session: max_turns,
                max_fix_attempts: 3,
                verbose,
            };

            info!(adapter = %adapter.name(), "Using adapter");

            let result = runner::run_loop(&engine, adapter.as_ref(), &config).await?;

            info!(
                rounds = result.rounds,
                sessions = result.sessions_used.len(),
                "Task complete"
            );
        }

        Command::Resume {
            task_id,
            adapter: adapter_name,
            skills_dir,
            max_turns,
            verbose,
        } => {
            init_tracing(verbose);

            let engine = StorageEngine::default_root().map_err(error::CliError::Storage)?;
            engine.init().map_err(error::CliError::Storage)?;

            let adapter = create_adapter(&adapter_name)
                .ok_or_else(|| error::CliError::UnknownAdapter(adapter_name.clone()))?;

            let tasks = engine.task_list().map_err(error::CliError::Storage)?;
            let task_entry = tasks.iter().find(|t| t.id == task_id).ok_or_else(|| {
                error::CliError::Storage(kerrigan_storage::StorageError::TaskNotFound(
                    task_id.clone(),
                ))
            })?;

            let skills_dir = skills_dir
                .map(std::path::PathBuf::from)
                .or_else(|| Some(engine.skills_dir()));

            let config = RunConfig {
                task_id: task_id.clone(),
                task_summary: task_entry.summary.clone(),
                workdir: std::path::PathBuf::from(&task_entry.workdir),
                skills_dir,
                shared_dir: Some(engine.shared_dir()),
                max_turns_per_session: max_turns,
                max_fix_attempts: 3,
                verbose,
            };

            info!(task = %task_entry.summary, id = %task_id, "Resuming task");
            info!(adapter = %adapter.name(), "Using adapter");

            let result = runner::run_loop(&engine, adapter.as_ref(), &config).await?;

            info!(
                rounds = result.rounds,
                sessions = result.sessions_used.len(),
                "Task complete"
            );
        }

        Command::Status { task_id } => {
            let engine = StorageEngine::default_root().map_err(error::CliError::Storage)?;

            if let Some(id) = task_id {
                let nodes = engine.list_all(&id).map_err(error::CliError::Storage)?;
                println!("{}", serde_json::to_string_pretty(&nodes).unwrap());
            } else {
                let tasks = engine.task_list().map_err(error::CliError::Storage)?;
                println!("{}", serde_json::to_string_pretty(&tasks).unwrap());
            }
        }

        Command::Tell { task_id, message } => {
            let engine = StorageEngine::default_root().map_err(error::CliError::Storage)?;
            engine
                .board_post(&task_id, &message)
                .map_err(error::CliError::Storage)?;
            println!("{{\"ok\": true}}");
        }
    }

    Ok(())
}
