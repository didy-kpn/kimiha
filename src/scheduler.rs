use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{
    error::OrchestratorError,
    event_bus::EventBus,
    task_id::TaskId,
    task_registry::TaskRegistry,
    types::{BackgroundTask, EventTask, EventType},
};

pub struct Scheduler<E, M> {
    task_registry: TaskRegistry<E, M>,
    background_task_ids: Vec<TaskId>,
    event_task_ids: Vec<TaskId>,
    event_bus: EventBus<E, M>,
}

impl<E: EventType + 'static + ToString, M: Clone + Send + 'static> Scheduler<E, M> {
    pub fn new(event_bus: EventBus<E, M>) -> Self {
        Self {
            task_registry: TaskRegistry::new(),
            background_task_ids: Vec::new(),
            event_task_ids: Vec::new(),
            event_bus,
        }
    }

    pub fn event_bus(&self) -> &EventBus<E, M> {
        &self.event_bus
    }

    pub fn task_registry(&self) -> &TaskRegistry<E, M> {
        &self.task_registry
    }

    pub fn register_background_task(&mut self, task: Arc<Mutex<dyn BackgroundTask>>) -> TaskId {
        let id = self.task_registry.register_background_task(task);
        self.background_task_ids.push(id.clone());
        id
    }

    pub fn register_event_task(&mut self, task: Arc<Mutex<dyn EventTask<E, M>>>) -> TaskId {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        channel_config::ChannelConfig,
        error::OrchestratorError,
        event_bus::EventBus,
        types::{BackgroundTask, EventTask, EventType, Executable},
    };
    use async_trait::async_trait;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[derive(Debug, Clone, Hash, PartialEq, Eq)]
    enum TestEvent {
        TestEvent1,
        TestEvent2,
    }

    impl EventType for TestEvent {}

    impl std::fmt::Display for TestEvent {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                TestEvent::TestEvent1 => write!(f, "TestEvent1"),
                TestEvent::TestEvent2 => write!(f, "TestEvent2"),
            }
        }
    }

    struct MockBackgroundTask {
        name: String,
        initialized: bool,
        executed: bool,
        shutdown: bool,
    }

    impl MockBackgroundTask {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                initialized: false,
                executed: false,
                shutdown: false,
            }
        }
    }

    #[async_trait]
    impl Executable for MockBackgroundTask {
        fn name(&self) -> &str {
            &self.name
        }

        async fn initialize(&mut self) -> Result<(), OrchestratorError> {
            self.initialized = true;
            Ok(())
        }

        async fn shutdown(&mut self) -> Result<(), OrchestratorError> {
            self.shutdown = true;
            Ok(())
        }
    }

    #[async_trait]
    impl BackgroundTask for MockBackgroundTask {
        async fn execute(&mut self) -> Result<(), OrchestratorError> {
            self.executed = true;
            Ok(())
        }
    }

    struct MockEventTask {
        name: String,
        event: TestEvent,
        initialized: bool,
        handled_event: Option<String>,
        shutdown: bool,
    }

    impl MockEventTask {
        fn new(name: &str, event: TestEvent) -> Self {
            Self {
                name: name.to_string(),
                event,
                initialized: false,
                handled_event: None,
                shutdown: false,
            }
        }
    }

    #[async_trait]
    impl Executable for MockEventTask {
        fn name(&self) -> &str {
            &self.name
        }

        async fn initialize(&mut self) -> Result<(), OrchestratorError> {
            self.initialized = true;
            Ok(())
        }

        async fn shutdown(&mut self) -> Result<(), OrchestratorError> {
            self.shutdown = true;
            Ok(())
        }
    }

    #[async_trait]
    impl EventTask<TestEvent, String> for MockEventTask {
        fn subscribed_event(&self) -> &TestEvent {
            &self.event
        }

        async fn handle_event(&mut self, event: String) -> Result<(), OrchestratorError> {
            self.handled_event = Some(event);
            Ok(())
        }
    }

    fn create_test_event_bus() -> EventBus<TestEvent, String> {
        let configs = vec![
            (
                TestEvent::TestEvent1,
                ChannelConfig::new(10, "Test Channel 1".to_string()),
            ),
            (
                TestEvent::TestEvent2,
                ChannelConfig::new(10, "Test Channel 2".to_string()),
            ),
        ];
        EventBus::new(configs)
    }

    #[tokio::test]
    async fn test_scheduler_new() {
        let event_bus = create_test_event_bus();
        let scheduler = Scheduler::<TestEvent, String>::new(event_bus);

        assert_eq!(scheduler.background_task_ids.len(), 0);
        assert_eq!(scheduler.event_task_ids.len(), 0);
        assert_eq!(scheduler.event_bus().channel_count(), 2);
    }

    #[tokio::test]
    async fn test_register_background_task() {
        let event_bus = create_test_event_bus();
        let mut scheduler = Scheduler::<TestEvent, String>::new(event_bus);

        let task = Arc::new(Mutex::new(MockBackgroundTask::new("Background Task 1")));
        let id = scheduler.register_background_task(task.clone());

        assert_eq!(scheduler.background_task_ids.len(), 1);
        assert_eq!(scheduler.background_task_ids[0], id);

        // Verify task is retrievable from registry
        let stored_task = scheduler.task_registry.get_background_task(&id);
        assert!(stored_task.is_some());
    }

    #[tokio::test]
    async fn test_register_event_task() {
        let event_bus = create_test_event_bus();
        let mut scheduler = Scheduler::<TestEvent, String>::new(event_bus);

        let task = Arc::new(Mutex::new(MockEventTask::new(
            "Event Task 1",
            TestEvent::TestEvent1,
        )));
        let id = scheduler.register_event_task(task.clone());

        assert_eq!(scheduler.event_task_ids.len(), 1);
        assert_eq!(scheduler.event_task_ids[0], id);

        // Verify task is retrievable from registry
        let stored_task = scheduler.task_registry.get_event_task(&id);
        assert!(stored_task.is_some());
    }

    #[tokio::test]
    async fn test_start_and_shutdown() {
        let event_bus = create_test_event_bus();
        let mut scheduler = Scheduler::<TestEvent, String>::new(event_bus);

        // Register background task
        let bg_task = Arc::new(Mutex::new(MockBackgroundTask::new("Background Task 1")));
        scheduler.register_background_task(bg_task.clone());

        // Register event task
        let event_task = Arc::new(Mutex::new(MockEventTask::new(
            "Event Task 1",
            TestEvent::TestEvent1,
        )));
        scheduler.register_event_task(event_task.clone());

        // Start the scheduler
        scheduler.start().await.expect("Failed to start scheduler");

        // Allow some time for tasks to initialize
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Check that tasks were initialized
        assert!(bg_task.lock().await.initialized);
        assert!(event_task.lock().await.initialized);

        // Publish an event to be handled by the event task
        let sender = scheduler
            .event_bus()
            .clone_sender(&TestEvent::TestEvent1)
            .unwrap();
        let _ = sender.send("test_event_data".to_string());

        // Allow time for event to be processed
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Verify background task executed (might have run multiple times)
        assert!(bg_task.lock().await.executed);

        // Shutdown the scheduler
        scheduler
            .shutdown()
            .await
            .expect("Failed to shutdown scheduler");

        // Verify shutdown was called on all tasks
        assert!(bg_task.lock().await.shutdown);
        assert!(event_task.lock().await.shutdown);

        // Wait a bit to ensure event is processed
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Check if event was handled (note: in a real scenario, this might be flaky due to timing)
        let event_task_lock = event_task.lock().await;
        assert!(event_task_lock.handled_event.is_some());
        if let Some(event_data) = &event_task_lock.handled_event {
            assert_eq!(event_data, "test_event_data");
        }
    }
}
