mod cli;

use clap::Parser;
use cli::{Cli, Command, DagCommand, TaskCommand};
use kerrigan::{NewNode, NodeUpdate, RegistryEngine, TaskEngine, output_json};
use std::io::Read;
use std::process;

fn find_task_for_node(registry: &RegistryEngine, node_id: &str) -> kerrigan::Result<String> {
    let tasks = registry.task_list()?;
    for task in &tasks {
        let task_dir = registry.task_dir(&task.id);
        if let Ok(te) = TaskEngine::open(&task_dir)
            && te.get_node(node_id).is_ok()
        {
            return Ok(task.id.clone());
        }
    }
    Err(kerrigan::Error::NodeNotFound(node_id.to_string()))
}

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli.command) {
        eprintln!("{}", serde_json::json!({"error": e.to_string()}));
        process::exit(1);
    }
}

fn run(command: Command) -> kerrigan::Result<()> {
    match command {
        Command::Init => {
            let reg = RegistryEngine::default_root()?;
            reg.init()?;
            output_json(serde_json::json!({"ok": true}));
        }

        Command::Task(sub) => {
            let reg = RegistryEngine::default_root()?;
            match sub {
                TaskCommand::Create { summary, workdir } => {
                    reg.init()?;
                    let id = reg.task_create(&summary, &workdir)?;
                    output_json(serde_json::json!({"task_id": id}));
                }
                TaskCommand::List => {
                    let tasks = reg.task_list()?;
                    output_json(&tasks);
                }
            }
        }

        Command::Dag(sub) => {
            let reg = RegistryEngine::default_root()?;
            match sub {
                DagCommand::Add {
                    task,
                    summary,
                    description,
                    depends_on,
                } => {
                    let task_dir = reg.task_dir(&task);
                    let te = kerrigan::TaskEngine::open(&task_dir)?;
                    let deps: Vec<String> =
                        depends_on.into_iter().filter(|s| !s.is_empty()).collect();
                    let id = te.add_node(
                        NewNode {
                            summary,
                            description,
                        },
                        deps,
                    )?;
                    output_json(serde_json::json!({"node_id": id}));
                }
                DagCommand::List { task } => {
                    let task_dir = reg.task_dir(&task);
                    let te = kerrigan::TaskEngine::open(&task_dir)?;
                    let nodes = te.list_all()?;
                    output_json(&nodes);
                }
                DagCommand::Ready { task } => {
                    let task_dir = reg.task_dir(&task);
                    let te = kerrigan::TaskEngine::open(&task_dir)?;
                    let nodes = te.list_ready()?;
                    output_json(&nodes);
                }
                DagCommand::Get { task, node } => {
                    let task_dir = reg.task_dir(&task);
                    let te = kerrigan::TaskEngine::open(&task_dir)?;
                    let n = te.get_node(&node)?;
                    output_json(&n);
                }
                DagCommand::Update { task, input } => {
                    let json = match input {
                        Some(s) => s,
                        None => {
                            let mut buf = String::new();
                            std::io::stdin()
                                .read_to_string(&mut buf)
                                .map_err(|e| kerrigan::Error::io("<stdin>", e))?;
                            buf
                        }
                    };
                    let updates: Vec<NodeUpdate> = serde_json::from_str(&json)
                        .map_err(|e| kerrigan::Error::json("<input>", e))?;
                    let task_dir = reg.task_dir(&task);
                    let te = kerrigan::TaskEngine::open(&task_dir)?;
                    te.update_nodes(updates)?;
                    output_json(serde_json::json!({"ok": true}));
                }
            }
        }

        Command::Status { task_id } => {
            let reg = RegistryEngine::default_root()?;
            if let Some(id) = task_id {
                let task_dir = reg.task_dir(&id);
                let te = kerrigan::TaskEngine::open(&task_dir)?;
                let nodes = te.list_all()?;
                output_json(&nodes);
            } else {
                let tasks = reg.task_list()?;
                output_json(&tasks);
            }
        }

        Command::Exec {
            task,
            node,
            adapter,
            detach,
        } => {
            kerrigan::exec::run(&task, &node, &adapter, detach)?;
        }
        Command::Send {
            node,
            message,
            interrupt,
        } => {
            kerrigan::send::run(&node, &message, interrupt)?;
        }

        Command::Stop { node } => {
            let reg = RegistryEngine::default_root()?;
            let task_id = find_task_for_node(&reg, &node)?;
            let task_dir = reg.task_dir(&task_id);
            let te = kerrigan::TaskEngine::open(&task_dir)?;
            if let Some(proc) = te.process_get_by_node(&node)? {
                if proc.status == kerrigan::ProcessStatus::Running {
                    unsafe {
                        libc::kill(proc.pid as i32, libc::SIGTERM);
                    }
                    te.process_mark_killed(proc.id)?;
                    output_json(serde_json::json!({"stopped": true, "pid": proc.pid}));
                } else {
                    output_json(
                        serde_json::json!({"stopped": false, "reason": "no running process"}),
                    );
                }
            } else {
                output_json(serde_json::json!({"stopped": false, "reason": "no process found"}));
            }
        }
        Command::Watch { task_id, interval } => {
            kerrigan::watch::run(&task_id, interval)?;
        }
        Command::Viz {
            port,
            host,
            static_dir,
        } => {
            let dir = if static_dir.is_empty() {
                let home = std::env::var("HOME").unwrap_or_default();
                std::path::PathBuf::from(home).join(".kerrigan").join("viz")
            } else {
                std::path::PathBuf::from(static_dir)
            };
            kerrigan::viz::run(&host, port, dir)?;
        }
    }

    Ok(())
}
