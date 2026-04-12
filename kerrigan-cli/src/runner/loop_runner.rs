use crate::adapter::session::generate_session_id;
use crate::adapter::traits::{AgentAdapter, AgentOutput};
use crate::error::CliError;
use crate::runner::board::BoardWatcher;
use crate::runner::templates;
use crate::runner::validate;
use kerrigan_storage::StorageEngine;

/// Configuration for the main loop.
pub struct RunConfig {
    /// Task ID to execute
    pub task_id: String,
    /// Task summary (for prompt injection)
    pub task_summary: String,
    /// Working directory for agent execution
    pub workdir: std::path::PathBuf,
    /// Path to the skills directory
    pub skills_dir: Option<std::path::PathBuf>,
    /// Path to the shared directory (guide files)
    pub shared_dir: Option<std::path::PathBuf>,
    /// Max continue turns before rotating session (default: 10)
    pub max_turns_per_session: usize,
    /// Max fix attempts per round (default: 3)
    pub max_fix_attempts: usize,
    /// Verbosity level (0=quiet, 1=info, 2=debug with agent output)
    pub verbose: u8,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            task_id: String::new(),
            task_summary: String::new(),
            workdir: std::path::PathBuf::from("."),
            skills_dir: None,
            shared_dir: None,
            max_turns_per_session: 10,
            max_fix_attempts: 3,
            verbose: 0,
        }
    }
}

/// Result of a single run execution.
pub struct RunResult {
    pub rounds: usize,
    pub sessions_used: Vec<String>,
}

/// Execute the main Kerrigan loop.
pub async fn run_loop(
    engine: &StorageEngine,
    adapter: &dyn AgentAdapter,
    config: &RunConfig,
) -> Result<RunResult, CliError> {
    let task_dir = engine.task_dir(&config.task_id);
    let mut board = BoardWatcher::new(&task_dir);
    let mut session_id = generate_session_id();
    let mut sessions_used = vec![session_id.clone()];
    let mut round: usize = 0;
    let mut turns_in_session: usize = 0;
    let mut is_first_round = true;

    // Auto-create origin node if DAG is empty
    let origin_node_id = if engine.list_all(&config.task_id)?.is_empty() {
        let id = engine.add_node(
            &config.task_id,
            kerrigan_storage::NewNode {
                summary: format!("Research: {}", config.task_summary),
                description: config.task_summary.clone(),
            },
            vec![],
        )?;
        Some(id)
    } else {
        None
    };

    loop {
        // Check termination: no ready nodes
        let ready = engine.list_ready(&config.task_id)?;
        if ready.is_empty() {
            break;
        }

        round += 1;

        // Session rotation: after N turns, start a new session
        if turns_in_session >= config.max_turns_per_session {
            session_id = generate_session_id();
            sessions_used.push(session_id.clone());
            turns_in_session = 0;
            is_first_round = true; // use resume template
        }

        // Resolve skills path for prompt injection
        let skills_path = config
            .skills_dir
            .as_ref()
            .map(|d| {
                d.join("kerrigan-storage")
                    .join("SKILL.md")
                    .to_string_lossy()
                    .into_owned()
            })
            .unwrap_or_default();

        // Resolve guide path for prompt injection
        let guide_path = config
            .shared_dir
            .as_ref()
            .map(|d| d.join("guide.md").to_string_lossy().into_owned())
            .unwrap_or_default();

        // Build prompt
        let prompt = if let Some(ref origin_id) = origin_node_id
            && round == 1
        {
            // Very first round: init with origin node
            templates::render_init(&config.task_summary, origin_id, &skills_path, &guide_path)
        } else if is_first_round {
            // New session (after rotation or restart)
            templates::render_resume(
                &config.task_id,
                &config.task_summary,
                &ready,
                &skills_path,
                &guide_path,
            )
        } else {
            templates::render_continue(&config.task_id, round, &ready, &skills_path, &guide_path)
        };

        // Inject board content if changed
        let prompt = if let Some(board_content) = board.check_for_changes() {
            format!("{prompt}\n\n## Message from user\n\n{board_content}")
        } else {
            prompt
        };

        // Call agent
        let skills_dir = config.skills_dir.as_deref();
        let _output: AgentOutput = if is_first_round || turns_in_session == 0 {
            adapter
                .run(&prompt, &session_id, skills_dir, &config.workdir)
                .await?
        } else {
            adapter
                .resume(&session_id, &prompt, &config.workdir)
                .await?
        };

        is_first_round = false;
        turns_in_session += 1;

        // Validate DAG consistency
        let mut fix_attempts = 0;
        loop {
            let issues = validate::validate_dag(engine, &config.task_id);
            if issues.is_empty() {
                break;
            }

            fix_attempts += 1;
            if fix_attempts > config.max_fix_attempts {
                eprintln!(
                    "Warning: DAG still has {} issues after {} fix attempts, proceeding anyway",
                    issues.len(),
                    config.max_fix_attempts
                );
                break;
            }

            let fix_prompt = templates::render_fix(&issues);
            adapter
                .resume(&session_id, &fix_prompt, &config.workdir)
                .await?;
            turns_in_session += 1;
        }
    }

    Ok(RunResult {
        rounds: round,
        sessions_used,
    })
}
