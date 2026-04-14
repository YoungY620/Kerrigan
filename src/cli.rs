use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "kerrigan",
    about = "DAG-based multi-agent orchestration CLI",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Initialize ~/.kerrigan/ directory structure (idempotent)
    Init,

    /// Manage tasks
    #[command(subcommand)]
    Task(TaskCommand),

    /// DAG operations
    #[command(subcommand)]
    Dag(DagCommand),

    /// Execute a node (worker wrapper)
    Exec {
        #[arg(long)]
        task: String,
        #[arg(long)]
        node: String,
        #[arg(long, default_value = "claude")]
        adapter: String,
        /// Run as detached background process
        #[arg(long)]
        detach: bool,
    },

    /// Send a message to a node's agent
    Send {
        #[arg(long)]
        node: String,
        #[arg(long)]
        message: String,
        /// Interrupt the running agent before delivering
        #[arg(long)]
        interrupt: bool,
    },

    /// Stop a running worker
    Stop {
        #[arg(long)]
        node: String,
    },

    /// Stream DAG events (for Monitor integration)
    Watch {
        /// Task ID
        task_id: String,
        #[arg(long, default_value = "2")]
        interval: u64,
    },

    /// Show task/DAG status
    Status {
        /// Task ID (omit to list all tasks)
        task_id: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum TaskCommand {
    /// Create a new task
    Create {
        #[arg(long)]
        summary: String,
        #[arg(long, default_value = ".")]
        workdir: String,
    },
    /// List all tasks
    List,
}

#[derive(Subcommand)]
pub enum DagCommand {
    /// Add a node to a task's DAG
    Add {
        #[arg(long)]
        task: String,
        #[arg(long)]
        summary: String,
        #[arg(long, default_value = "")]
        description: String,
        #[arg(long, default_value = "", value_delimiter = ',')]
        depends_on: Vec<String>,
    },
    /// List all nodes
    List {
        #[arg(long)]
        task: String,
    },
    /// List ready nodes
    Ready {
        #[arg(long)]
        task: String,
    },
    /// Get full node details
    Get {
        #[arg(long)]
        task: String,
        #[arg(long)]
        node: String,
    },
    /// Batch update nodes
    Update {
        #[arg(long)]
        task: String,
        /// JSON input. Reads from stdin if omitted.
        #[arg(long)]
        input: Option<String>,
    },
}
