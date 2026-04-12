mod cli;

use clap::Parser;
use cli::{Cli, Command, TaskCommand};
use kerrigan_storage::{NewNode, NodeUpdate, StorageEngine};
use std::io::Read;
use std::process;

fn main() {
    let cli = Cli::parse();

    let engine = match StorageEngine::default_root() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    };

    if let Err(e) = run(engine, cli.command) {
        eprintln!("error: {e}");
        process::exit(1);
    }
}

fn run(engine: StorageEngine, command: Command) -> kerrigan_storage::Result<()> {
    match command {
        Command::Init => {
            engine.init()?;
            println!("{{\"ok\": true}}");
        }

        Command::Task(sub) => match sub {
            TaskCommand::Create { summary, workdir } => {
                let id = engine.task_create(&summary, &workdir)?;
                println!("{}", serde_json::json!({"task_id": id}));
            }
            TaskCommand::List => {
                let tasks = engine.task_list()?;
                println!("{}", serde_json::to_string_pretty(&tasks).unwrap());
            }
        },

        Command::Add {
            task,
            summary,
            description,
            depends_on,
        } => {
            let deps: Vec<String> = depends_on.into_iter().filter(|s| !s.is_empty()).collect();
            let id = engine.add_node(
                &task,
                NewNode {
                    summary,
                    description,
                },
                deps,
            )?;
            println!("{}", serde_json::json!({"node_id": id}));
        }

        Command::List { task } => {
            let nodes = engine.list_all(&task)?;
            println!("{}", serde_json::to_string_pretty(&nodes).unwrap());
        }

        Command::Ready { task } => {
            let nodes = engine.list_ready(&task)?;
            println!("{}", serde_json::to_string_pretty(&nodes).unwrap());
        }

        Command::Update { task, input } => {
            let json = match input {
                Some(s) => s,
                None => {
                    let mut buf = String::new();
                    std::io::stdin()
                        .read_to_string(&mut buf)
                        .map_err(|e| kerrigan_storage::StorageError::io("<stdin>", e))?;
                    buf
                }
            };
            let updates: Vec<NodeUpdate> = serde_json::from_str(&json)
                .map_err(|e| kerrigan_storage::StorageError::json("<input>", e))?;
            engine.update_nodes(&task, updates)?;
            println!("{{\"ok\": true}}");
        }

        Command::Get { task, node } => {
            let n = engine.get_node(&task, &node)?;
            println!("{}", serde_json::to_string_pretty(&n).unwrap());
        }

        Command::Tell { task, message } => {
            engine.board_post(&task, &message)?;
            println!("{{\"ok\": true}}");
        }
    }
    Ok(())
}
