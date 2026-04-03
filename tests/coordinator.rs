//! Tests for the Coordinator module

use open_cc::coordinator::{Coordinator, SubAgentTask, TaskStatus};

#[tokio::test]
async fn test_coordinator_new() {
    let (coordinator, _rx) = Coordinator::new();
    // Just verify it constructs without panic
    let tasks = coordinator.list_tasks().await;
    assert!(tasks.is_empty());
}

#[tokio::test]
async fn test_coordinator_spawn_task() {
    let (coordinator, _rx) = Coordinator::new();

    let task = SubAgentTask {
        id: "task-1".to_string(),
        prompt: "Test prompt".to_string(),
        model: None,
        max_tokens: None,
        status: TaskStatus::Pending,
        result: None,
        error: None,
    };

    let id = coordinator.spawn(task).await;
    assert_eq!(id, "task-1");

    let retrieved = coordinator.get_task("task-1").await;
    assert!(retrieved.is_some());
    let t = retrieved.unwrap();
    assert_eq!(t.id, "task-1");
    assert_eq!(t.prompt, "Test prompt");
    assert_eq!(t.status, TaskStatus::Pending);
}

#[tokio::test]
async fn test_coordinator_get_nonexistent() {
    let (coordinator, _rx) = Coordinator::new();
    let result = coordinator.get_task("nonexistent").await;
    assert!(result.is_none());
}

#[tokio::test]
async fn test_coordinator_list_tasks() {
    let (coordinator, _rx) = Coordinator::new();

    for i in 0..3 {
        let task = SubAgentTask {
            id: format!("task-{}", i),
            prompt: format!("Prompt {}", i),
            model: None,
            max_tokens: None,
            status: TaskStatus::Pending,
            result: None,
            error: None,
        };
        coordinator.spawn(task).await;
    }

    let tasks = coordinator.list_tasks().await;
    assert_eq!(tasks.len(), 3);
}

#[tokio::test]
async fn test_coordinator_cancel_pending() {
    let (coordinator, _rx) = Coordinator::new();

    let task = SubAgentTask {
        id: "cancel-me".to_string(),
        prompt: "Cancel this".to_string(),
        model: None,
        max_tokens: None,
        status: TaskStatus::Pending,
        result: None,
        error: None,
    };
    coordinator.spawn(task).await;

    let cancelled = coordinator.cancel("cancel-me").await;
    assert!(cancelled);

    let task = coordinator.get_task("cancel-me").await.unwrap();
    assert_eq!(task.status, TaskStatus::Cancelled);
}

#[tokio::test]
async fn test_coordinator_cancel_running() {
    let (coordinator, _rx) = Coordinator::new();

    let task = SubAgentTask {
        id: "running".to_string(),
        prompt: "Running task".to_string(),
        model: None,
        max_tokens: None,
        status: TaskStatus::Running,
        result: None,
        error: None,
    };
    coordinator.spawn(task).await;

    let cancelled = coordinator.cancel("running").await;
    assert!(cancelled);

    let task = coordinator.get_task("running").await.unwrap();
    assert_eq!(task.status, TaskStatus::Cancelled);
}

#[tokio::test]
async fn test_coordinator_cancel_completed() {
    let (coordinator, _rx) = Coordinator::new();

    let task = SubAgentTask {
        id: "done".to_string(),
        prompt: "Done task".to_string(),
        model: None,
        max_tokens: None,
        status: TaskStatus::Completed,
        result: Some("output".to_string()),
        error: None,
    };
    coordinator.spawn(task).await;

    let cancelled = coordinator.cancel("done").await;
    assert!(!cancelled); // Cannot cancel completed tasks
}

#[tokio::test]
async fn test_coordinator_cancel_nonexistent() {
    let (coordinator, _rx) = Coordinator::new();
    let result = coordinator.cancel("nonexistent").await;
    assert!(!result);
}

#[tokio::test]
async fn test_coordinator_wait_all_empty() {
    let (coordinator, _rx) = Coordinator::new();
    let results = coordinator.wait_all().await;
    assert!(results.is_empty());
}

#[tokio::test]
async fn test_coordinator_wait_all_returns_results() {
    let (coordinator, _rx) = Coordinator::new();

    let task = SubAgentTask {
        id: "done-task".to_string(),
        prompt: "Done".to_string(),
        model: None,
        max_tokens: None,
        status: TaskStatus::Completed,
        result: Some("test result".to_string()),
        error: None,
    };
    coordinator.spawn(task).await;

    let results = coordinator.wait_all().await;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].output, "test result");
}

#[test]
fn test_task_status_all_variants() {
    // Verify all variants can be constructed
    let _statuses = [
        TaskStatus::Pending,
        TaskStatus::Running,
        TaskStatus::Completed,
        TaskStatus::Failed,
        TaskStatus::Cancelled,
    ];
}

#[test]
fn test_sub_agent_task_clone() {
    let task = SubAgentTask {
        id: "clone-test".to_string(),
        prompt: "Clone me".to_string(),
        model: Some("claude-sonnet-4".to_string()),
        max_tokens: Some(4096),
        status: TaskStatus::Pending,
        result: None,
        error: None,
    };
    let cloned = task.clone();
    assert_eq!(cloned.id, task.id);
    assert_eq!(cloned.prompt, task.prompt);
    assert_eq!(cloned.model, task.model);
}
