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
    #[error("Component not initialized: {0}")]
    NotInitialized(&'static str),
    #[error("Invalid task's eventtype: {0}")]
    InvalidTaskEventType(&'static str),
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
}
