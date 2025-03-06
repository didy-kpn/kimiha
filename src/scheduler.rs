use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{
    error::OrchestratorError,
    event_bus::EventBus,
    task_id::TaskId,
    task_registry::TaskRegistry,
    types::{BackgroundTask, EventTask, EventType},
};

pub struct Scheduler<E> {
    task_registry: TaskRegistry<E>,
    background_task_ids: Vec<TaskId>,
    event_task_ids: Vec<TaskId>,
    event_bus: EventBus<E>,
}

impl<E: EventType + 'static + ToString> Scheduler<E> {
    pub fn new(event_bus: EventBus<E>) -> Self {
        Self {
            task_registry: TaskRegistry::new(),
            background_task_ids: Vec::new(),
            event_task_ids: Vec::new(),
            event_bus,
        }
    }

    pub fn event_bus(&self) -> &EventBus<E> {
        &self.event_bus
    }

    pub fn register_background_task(&mut self, task: Arc<Mutex<dyn BackgroundTask>>) -> TaskId {
        let id = self.task_registry.register_background_task(task);
        self.background_task_ids.push(id.clone());
        id
    }

    pub fn register_event_task(&mut self, task: Arc<Mutex<dyn EventTask<E>>>) -> TaskId {
        let id = self.task_registry.register_event_task(task);
        self.event_task_ids.push(id.clone());
        id
    }

    pub async fn start(&mut self) -> Result<(), OrchestratorError> {
        for id in &self.background_task_ids {
            let task = self
                .task_registry
                .get_background_task(id)
                .ok_or_else(|| OrchestratorError::TaskNotFound(id.clone()))?;

            task.lock().await.initialize().await?;

            let task_clone = task.clone();
            tokio::spawn(async move {
                loop {
                    if let Err(e) = task_clone.lock().await.execute().await {
                        eprintln!("Background task execution error: {}", e);
                    }
                }
            });
        }

        for id in &self.event_task_ids {
            let task = self
                .task_registry
                .get_event_task(id)
                .ok_or_else(|| OrchestratorError::TaskNotFound(id.clone()))?;

            let mut task_lock = task.lock().await;
            task_lock.initialize().await?;
            let event = task_lock.subscribed_event().clone();
            drop(task_lock);

            if let Ok(mut rx) = self.event_bus.subscribe(&event) {
                let task_clone = task.clone();
                tokio::spawn(async move {
                    while let Ok(event_data) = rx.recv().await {
                        if let Err(e) = task_clone.lock().await.handle_event(event_data).await {
                            eprintln!("Event handling error: {}", e);
                        }
                    }
                });
            }
        }

        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<(), OrchestratorError> {
        for id in &self.background_task_ids {
            if let Some(task) = self.task_registry.get_background_task(id) {
                task.lock().await.shutdown().await?;
            }
        }

        for id in &self.event_task_ids {
            if let Some(task) = self.task_registry.get_event_task(id) {
                task.lock().await.shutdown().await?;
            }
        }

        Ok(())
    }
}
