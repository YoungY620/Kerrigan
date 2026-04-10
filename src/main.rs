#![allow(dead_code, unused_imports)]

pub mod adapter;
pub mod error;
pub mod runner;

use adapter::create_adapter;
use clap::Parser;
use kerrigan_storage::StorageEngine;
use runner::RunConfig;

#[derive(Parser)]
#[command(
    name = "kerrigan",
    about = "Kerrigan multi-agent orchestration CLI",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Run a task's DAG to completion using an agent adapter
    Run {
        /// Task ID to execute
        #[arg(help = "The task ID to run")]
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
    },
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
            task_id,
            adapter: adapter_name,
            skills_dir,
            max_turns,
        } => {
            let engine = StorageEngine::default_root().map_err(error::CliError::Storage)?;

            let adapter = create_adapter(&adapter_name)
                .ok_or_else(|| error::CliError::UnknownAdapter(adapter_name.clone()))?;

            // Get task summary from registry
            let tasks = engine.task_list()?;
            let task_entry = tasks.iter().find(|t| t.id == task_id).ok_or_else(|| {
                error::CliError::Storage(kerrigan_storage::StorageError::TaskNotFound(
                    task_id.clone(),
                ))
            })?;

            // Default skills_dir to ~/.kerrigan/skills/ (deployed by kerrigan-storage init)
            let skills_dir = skills_dir
                .map(std::path::PathBuf::from)
                .or_else(|| Some(engine.skills_dir()));

            let config = RunConfig {
                task_id: task_id.clone(),
                task_summary: task_entry.summary.clone(),
                workdir: std::path::PathBuf::from(&task_entry.workdir),
                skills_dir,
                max_turns_per_session: max_turns,
                max_fix_attempts: 3,
            };

            println!("Starting task: {} ({})", task_entry.summary, task_id);
            println!("Adapter: {}", adapter.name());

            let result = runner::run_loop(&engine, adapter.as_ref(), &config).await?;

            println!(
                "Task complete. {} rounds, {} sessions used.",
                result.rounds,
                result.sessions_used.len()
            );
        }
    }

    Ok(())
}
