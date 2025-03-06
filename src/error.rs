use crate::task_id::TaskId;

#[derive(thiserror::Error, Debug)]
pub enum OrchestratorError {
    #[error("Task initialization error: {0}")]
    TaskInitialization(String),
    #[error("Task execution error: {0}")]
    TaskExecution(String),
    #[error("Task shutdown error: {0}")]
    TaskShutdown(String),
    #[error("Invalid channel: {0}")]
    InvalidChannel(String),
    #[error("Event send error: {0}")]
    EventSend(String),
    #[error("Event receive error: {0}")]
    EventReceive(String),
    #[error("Task not found: {0}")]
    TaskNotFound(TaskId),
    #[error("Missing required component: {0}")]
    MissingComponent(&'static str),
}
