use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "kerrigan-storage",
    about = "DAG engine and storage management for Kerrigan multi-agent framework",
    version,
    after_help = "All node/task operations output JSON for machine consumption.\n\
                  Use `kerrigan-storage init` before any other command."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Initialize ~/.kerrigan/ directory structure (idempotent)
    Init,

    /// Manage tasks (create, list)
    #[command(subcommand)]
    Task(TaskCommand),

    /// Add a node to a task's DAG
    Add {
        /// Task ID
        #[arg(long, help = "The task ID to add the node to")]
        task: String,

        /// Short summary of the node
        #[arg(long, help = "Brief description of what this node does")]
        summary: String,

        /// Detailed description (markdown)
        #[arg(long, default_value = "", help = "Detailed description and references (markdown format)")]
        description: String,

        /// Comma-separated predecessor node IDs
        #[arg(long, default_value = "", value_delimiter = ',', help = "Node IDs this node depends on (comma-separated)")]
        depends_on: Vec<String>,
    },

    /// List all nodes in a task's DAG (JSON output)
    List {
        /// Task ID
        #[arg(long, help = "The task ID to list nodes for")]
        task: String,
    },

    /// List all Ready nodes in a task's DAG (JSON output)
    Ready {
        /// Task ID
        #[arg(long, help = "The task ID to list ready nodes for")]
        task: String,
    },

    /// Batch update nodes in a task's DAG (JSON input)
    Update {
        /// Task ID
        #[arg(long, help = "The task ID to update nodes in")]
        task: String,

        /// JSON input: array of NodeUpdate objects. Read from stdin if omitted.
        #[arg(long, help = "JSON array of updates, e.g. '[{\"id\":\"...\",\"status\":\"succeeded\"}]'. Reads from stdin if omitted.")]
        input: Option<String>,
    },

    /// Get full details of a single node (JSON output)
    Get {
        /// Task ID
        #[arg(long, help = "The task ID containing the node")]
        task: String,

        /// Node ID
        #[arg(long, help = "The node ID to retrieve")]
        node: String,
    },
}

#[derive(Subcommand)]
pub enum TaskCommand {
    /// Create a new task
    Create {
        /// Short summary of the task
        #[arg(long, help = "Brief description of the task")]
        summary: String,

        /// Working directory for the task
        #[arg(long, default_value = ".", help = "Working directory path for this task")]
        workdir: String,
    },

    /// List all registered tasks (JSON output)
    List,
}
