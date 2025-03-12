use std::{collections::HashMap, sync::Arc};

use tokio::sync::Mutex;

use crate::{
    task_id::TaskId,
    types::{BackgroundTask, EventTask, EventType},
};

pub struct TaskRegistry<E, M> {
    background_tasks: HashMap<TaskId, Arc<Mutex<dyn BackgroundTask>>>,
    event_tasks: HashMap<TaskId, Arc<Mutex<dyn EventTask<E, M>>>>,
}

impl<E: EventType + 'static + ToString, M: Clone + Send + 'static> Default for TaskRegistry<E, M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E: EventType + 'static + ToString, M: Clone + Send + 'static> TaskRegistry<E, M> {
    pub fn new() -> Self {
        Self {
            background_tasks: HashMap::new(),
            event_tasks: HashMap::new(),
        }
    }

    pub fn register_background_task(&mut self, task: Arc<Mutex<dyn BackgroundTask>>) -> TaskId {
        let id = TaskId::new();
        self.background_tasks.insert(id.clone(), task);
        id
    }

    pub fn register_event_task(&mut self, task: Arc<Mutex<dyn EventTask<E, M>>>) -> TaskId {
        let id = TaskId::new();
        self.event_tasks.insert(id.clone(), task);
        id
    }

    pub fn get_background_task(&self, id: &TaskId) -> Option<Arc<Mutex<dyn BackgroundTask>>> {
        self.background_tasks.get(id).cloned()
    }

    pub fn get_event_task(&self, id: &TaskId) -> Option<Arc<Mutex<dyn EventTask<E, M>>>> {
        self.event_tasks.get(id).cloned()
    }
}
