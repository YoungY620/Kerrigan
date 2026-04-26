use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "kerrigan",
    about = "DAG-based multi-agent orchestration CLI",
    long_about = "DAG-based multi-agent orchestration CLI.\n\n\
        All commands return JSON in an envelope format:\n  \
        {\"data\": <result>, \"reminder\": \"...\"}\n\n\
        Errors are printed to stderr as: {\"error\": \"<message>\"}\n\n\
        Typical workflow:\n  \
        1. kerrigan init\n  \
        2. kerrigan task create --summary \"...\" --workdir .\n  \
        3. kerrigan dag add --task <id> --summary \"...\" [--depends-on ...]\n  \
        4. kerrigan dag ready --task <id>        # find executable nodes\n  \
        5. kerrigan exec --task <id> --node <id> # run a node\n  \
        6. kerrigan dag update --task <id> ...   # record results",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Initialize ~/.kerrigan/ directory structure (idempotent)
    #[command(
        long_about = "Initialize ~/.kerrigan/ directory structure. Safe to run multiple times.\n\
            Must be run before any other kerrigan command.\n\
            Returns: {\"data\": {\"ok\": true}}"
    )]
    Init,

    /// Manage tasks (create, list)
    #[command(subcommand)]
    Task(TaskCommand),

    /// DAG operations (add, list, ready, get, update)
    #[command(subcommand)]
    Dag(DagCommand),

    /// Execute a node (worker wrapper)
    #[command(long_about = "Execute a node by spawning an agent worker.\n\
            Use --detach to run in the background (returns pid, session_id, log_path).\n\
            Without --detach, runs in the foreground and blocks until the agent exits.")]
    Exec {
        /// Task ID that contains the node
        #[arg(
            long,
            help = "Task ID (from 'kerrigan task create' or 'kerrigan task list')"
        )]
        task: String,
        /// Node ID to execute
        #[arg(
            long,
            help = "Node ID (from 'kerrigan dag ready' or 'kerrigan dag list')"
        )]
        node: String,
        /// Agent adapter to use
        #[arg(
            long,
            default_value = "claude",
            help = "Agent adapter: 'claude' or 'kimi'"
        )]
        adapter: String,
        /// Run as detached background process
        #[arg(long, help = "Run in background; returns {pid, session_id, log_path}")]
        detach: bool,
        /// Internal: session ID injected by detach parent — skips node setup
        #[arg(long, hide = true)]
        internal_session: Option<String>,
    },

    /// Send a message to a node's running agent
    #[command(long_about = "Send a message to a node's running agent process.\n\
            If no process is running, the message is queued for the next execution.\n\
            Use --interrupt to stop the current agent turn before delivering.")]
    Send {
        /// Node ID whose agent should receive the message
        #[arg(long, help = "Node ID (must have a running or pending agent)")]
        node: String,
        /// Message content to deliver
        #[arg(long, help = "Message text to send to the agent")]
        message: String,
        /// Interrupt the running agent before delivering
        #[arg(long, help = "Send SIGUSR1 to interrupt current agent turn first")]
        interrupt: bool,
    },

    /// Stop a running worker
    #[command(long_about = "Stop a running worker process for a node.\n\
            Sends SIGTERM to the agent process.\n\
            Returns: {\"data\": {\"stopped\": true/false, ...}}")]
    Stop {
        /// Node ID whose worker to stop
        #[arg(long, help = "Node ID with an active worker process")]
        node: String,
    },

    /// Stream DAG events for Monitor integration
    #[command(
        long_about = "Stream DAG events as JSONL (one JSON object per line).\n\
            Polls the task database and emits status changes, process deaths,\n\
            timeouts, and task completion events. Runs until the task completes\n\
            or is interrupted."
    )]
    Watch {
        /// Task ID to watch
        #[arg(help = "Task ID to monitor")]
        task_id: String,
        /// Polling interval in seconds
        #[arg(
            long,
            default_value = "2",
            help = "Polling interval in seconds (default: 2)"
        )]
        interval: u64,
    },

    /// Start the DAG visualization web server
    #[command(
        long_about = "Start a web server that visualizes all DAG tasks.\n\
            Serves the frontend at http://<host>:<port> and the API at /api/tasks.\n\
            Static files are read from <static-dir> (default: ./viz/out).\n\
            Run 'make viz-build' first to build the frontend."
    )]
    Viz {
        /// Port to listen on
        #[arg(short, long, default_value = "7070")]
        port: u16,
        /// Host/IP to bind (0.0.0.0 for all interfaces)
        #[arg(long, default_value = "0.0.0.0")]
        host: String,
        /// Path to the built frontend static files (default: ~/.kerrigan/viz)
        #[arg(long, default_value = "")]
        static_dir: String,
    },

    /// Show task/DAG status overview
    #[command(long_about = "Show status overview.\n\
            Without arguments: lists all registered tasks.\n\
            With a task ID: shows all nodes in that task's DAG.\n\
            Returns: {\"data\": [...]}")]
    Status {
        /// Task ID (omit to list all tasks)
        #[arg(help = "Task ID to inspect (omit to list all tasks)")]
        task_id: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum TaskCommand {
    /// Create a new task
    #[command(
        long_about = "Create a new task and its database.\n\
            Returns: {\"data\": {\"task_id\": \"<uuid>\"}}\n\
            The task_id is used in all subsequent dag commands.",
        after_help = "EXAMPLES:\n  \
            kerrigan task create --summary \"Refactor auth module\" --workdir /home/user/project\n  \
            kerrigan task create --summary \"Investigate slow queries\""
    )]
    Create {
        /// Brief description of the task's goal
        #[arg(long, help = "One-line summary of what this task aims to accomplish")]
        summary: String,
        /// Working directory for the task
        #[arg(
            long,
            default_value = ".",
            help = "Working directory for agent execution (default: current dir)"
        )]
        workdir: String,
    },
    /// List all registered tasks
    #[command(long_about = "List all registered tasks.\n\
            Returns: {\"data\": [{\"id\": \"...\", \"summary\": \"...\", \"workdir\": \"...\", \"created_at\": \"...\"}]}")]
    List,
}

#[derive(Subcommand)]
pub enum DagCommand {
    /// Add a node to a task's DAG
    #[command(
        long_about = "Add a new node to a task's DAG.\n\
            The node starts in 'pending' status and automatically transitions to 'ready'\n\
            when all its predecessors have succeeded (or immediately if it has no predecessors).\n\
            Returns: {\"data\": {\"node_id\": \"<uuid>\"}}",
        after_help = "EXAMPLES:\n  \
            kerrigan dag add --task abc-123 --summary \"Analyze auth flow\" \\\n    \
              --description \"Read all files under src/auth/ and document the call graph\"\n  \
            kerrigan dag add --task abc-123 --summary \"Implement fix\" --depends-on node1,node2"
    )]
    Add {
        /// Task ID that owns this DAG
        #[arg(long, help = "Task ID (from 'kerrigan task create')")]
        task: String,
        /// One-line summary starting with a verb
        #[arg(
            long,
            help = "One-line summary starting with a verb (e.g. 'Analyze auth flow')"
        )]
        summary: String,
        /// Detailed description of what needs to be done
        #[arg(
            long,
            default_value = "",
            help = "Detailed task specification: objective, context, scope, constraints, output requirements"
        )]
        description: String,
        /// Comma-separated predecessor node IDs
        #[arg(
            long,
            default_value = "",
            value_delimiter = ',',
            help = "Comma-separated node IDs that must succeed before this node can execute"
        )]
        depends_on: Vec<String>,
        /// Comma-separated successor node IDs (existing nodes that should depend on this node)
        #[arg(
            long,
            default_value = "",
            value_delimiter = ',',
            help = "Comma-separated node IDs that will depend on this node (reverse dependency)"
        )]
        successor: Vec<String>,
        /// Node I/O contract (required). JSON object with two keys:
        ///   predecessor: { "<pred_node_id>": <JSON Schema> } — one entry per depends-on node
        ///   output: <JSON Schema> — schema that result_summary must satisfy on success
        #[arg(long, help = "JSON NodeContract {predecessor:{\"<id>\":<schema>,...}, output:<schema>}")]
        contract: String,
    },
    /// List all nodes in a task's DAG
    #[command(long_about = "List all nodes in a task's DAG with summary info.\n\
            Returns: {\"data\": [{\"id\": \"...\", \"summary\": \"...\", \"successors\": [...], \"status\": \"...\"}]}")]
    List {
        /// Task ID
        #[arg(long, help = "Task ID to list nodes for")]
        task: String,
    },
    /// List nodes ready for execution
    #[command(
        long_about = "List nodes that are ready to execute (all predecessors have succeeded).\n\
            Returns the same format as 'dag list' but filtered to only ready nodes.\n\
            Returns: {\"data\": [{\"id\": \"...\", \"summary\": \"...\", \"successors\": [...], \"status\": \"ready\"}]}"
    )]
    Ready {
        /// Task ID
        #[arg(long, help = "Task ID to check for ready nodes")]
        task: String,
    },
    /// Get full node details including predecessors and metadata
    #[command(
        long_about = "Get full details for a specific node, including predecessors, successors, and metadata.\n\
            Returns: {\"data\": {\"id\": \"...\", \"summary\": \"...\", \"description\": \"...\",\n\
            \"predecessors\": [...], \"successors\": [...], \"status\": \"...\", \"metadata\": {...}}}\n\n\
            Use this to read predecessor result_summary values before executing a node."
    )]
    Get {
        /// Task ID
        #[arg(long, help = "Task ID containing the node")]
        task: String,
        /// Node ID
        #[arg(long, help = "Node ID to retrieve")]
        node: String,
    },
    /// Batch update node status and metadata
    #[command(
        long_about = "Batch update one or more nodes' status and/or metadata.\n\
            Input is a JSON array of updates. Each update has:\n  \
            - id (required): node ID\n  \
            - status (optional): \"running\", \"succeeded\", or \"failed\"\n  \
            - metadata (optional): {result_summary, failure_reason, spawned_tasks, session_ids}\n\n\
            Valid status transitions: ready->running, running->succeeded, running->failed.\n\
            Returns: {\"data\": {\"ok\": true}}",
        after_help = "EXAMPLES:\n  \
            # Mark succeeded with result:\n  \
            kerrigan dag update --task abc-123 --input \\\n    \
              '[{\"id\":\"n1\",\"status\":\"succeeded\",\"metadata\":{\"result_summary\":\"Found 3 endpoints\"}}]'\n\n  \
            # Mark failed:\n  \
            kerrigan dag update --task abc-123 --input \\\n    \
              '[{\"id\":\"n2\",\"status\":\"failed\",\"metadata\":{\"failure_reason\":\"DB connection refused\"}}]'\n\n  \
            # Read from stdin:\n  \
            echo '[{\"id\":\"n1\",\"status\":\"succeeded\"}]' | kerrigan dag update --task abc-123"
    )]
    Update {
        /// Task ID
        #[arg(long, help = "Task ID containing the nodes to update")]
        task: String,
        /// JSON array of updates. Reads from stdin if omitted.
        #[arg(
            long,
            help = "JSON array: [{\"id\":\"<node>\",\"status\":\"succeeded\",\"metadata\":{\"result_summary\":\"...\"}}]. Reads stdin if omitted"
        )]
        input: Option<String>,
    },
}
