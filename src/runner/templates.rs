use crate::runner::validate::DagIssue;
use kerrigan_storage::NodeSummary;
use minijinja::{Environment, context};

// Templates compiled into the binary
const INIT_TEMPLATE: &str = include_str!("templates/init.md");
const CONTINUE_TEMPLATE: &str = include_str!("templates/continue.md");
const RESUME_TEMPLATE: &str = include_str!("templates/resume.md");
const FIX_TEMPLATE: &str = include_str!("templates/fix.md");

fn env() -> Environment<'static> {
    let mut env = Environment::new();
    env.add_template("init", INIT_TEMPLATE).unwrap();
    env.add_template("continue", CONTINUE_TEMPLATE).unwrap();
    env.add_template("resume", RESUME_TEMPLATE).unwrap();
    env.add_template("fix", FIX_TEMPLATE).unwrap();
    env
}

/// Render the init template (first round of a new task).
pub fn render_init(task_id: &str, task_summary: &str, ready_nodes: &[NodeSummary]) -> String {
    let env = env();
    let tmpl = env.get_template("init").unwrap();
    tmpl.render(context! {
        task_id => task_id,
        task_summary => task_summary,
        ready_nodes => ready_nodes.iter().map(|n| context! {
            id => &n.id,
            summary => &n.summary,
        }).collect::<Vec<_>>(),
    })
    .unwrap()
}

/// Render the continue template (subsequent rounds, same session).
pub fn render_continue(task_id: &str, round: usize, ready_nodes: &[NodeSummary]) -> String {
    let env = env();
    let tmpl = env.get_template("continue").unwrap();
    tmpl.render(context! {
        task_id => task_id,
        round => round,
        ready_nodes => ready_nodes.iter().map(|n| context! {
            id => &n.id,
            summary => &n.summary,
        }).collect::<Vec<_>>(),
    })
    .unwrap()
}

/// Render the resume template (new session after rotation or restart).
pub fn render_resume(task_id: &str, task_summary: &str, ready_nodes: &[NodeSummary]) -> String {
    let env = env();
    let tmpl = env.get_template("resume").unwrap();
    tmpl.render(context! {
        task_id => task_id,
        task_summary => task_summary,
        ready_nodes => ready_nodes.iter().map(|n| context! {
            id => &n.id,
            summary => &n.summary,
        }).collect::<Vec<_>>(),
    })
    .unwrap()
}

/// Render the fix template (DAG validation failed).
pub fn render_fix(issues: &[DagIssue]) -> String {
    let env = env();
    let tmpl = env.get_template("fix").unwrap();
    tmpl.render(context! {
        issues => issues.iter().map(|i| context! {
            node_id => &i.node_id,
            description => &i.description,
        }).collect::<Vec<_>>(),
    })
    .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use kerrigan_storage::NodeStatus;

    fn sample_nodes() -> Vec<NodeSummary> {
        vec![
            NodeSummary {
                id: "n1".into(),
                summary: "Analyze code".into(),
                successors: vec![],
                status: NodeStatus::Ready,
            },
            NodeSummary {
                id: "n2".into(),
                summary: "Write tests".into(),
                successors: vec![],
                status: NodeStatus::Ready,
            },
        ]
    }

    #[test]
    fn init_template_renders() {
        let result = render_init("task-abc", "Refactor auth module", &sample_nodes());
        assert!(result.contains("Refactor auth module"));
        assert!(result.contains("kerrigan-storage"));
        assert!(result.contains("n1"));
        assert!(result.contains("Analyze code"));
        assert!(result.contains("n2"));
        assert!(result.contains("Write tests"));
    }

    #[test]
    fn continue_template_renders() {
        let result = render_continue("task-abc", 3, &sample_nodes());
        assert!(result.contains("Round 3"));
        assert!(result.contains("n1"));
    }

    #[test]
    fn resume_template_renders() {
        let result = render_resume("task-abc", "Refactor auth", &sample_nodes());
        assert!(result.contains("new session"));
        assert!(result.contains("Refactor auth"));
        assert!(result.contains("n1"));
    }

    #[test]
    fn fix_template_renders() {
        let issues = vec![
            DagIssue {
                node_id: "n1".into(),
                description: "Missing result_summary".into(),
            },
            DagIssue {
                node_id: "n2".into(),
                description: "Empty summary".into(),
            },
        ];
        let result = render_fix(&issues);
        assert!(result.contains("n1"));
        assert!(result.contains("Missing result_summary"));
        assert!(result.contains("n2"));
    }
}
