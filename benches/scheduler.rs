use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::runtime::Runtime;
use async_trait::async_trait;

use kimiha::{
    channel_config::ChannelConfig,
    error::OrchestratorError,
    event_bus::EventBus,
    scheduler::Scheduler,
    types::{BackgroundTask, EventTask, EventType, Executable},
};

// Define test events for benchmarking
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
enum BenchEvent {
    Event1,
    Event2,
    Event3,
}

impl EventType for BenchEvent {}

impl ToString for BenchEvent {
    fn to_string(&self) -> String {
        match self {
            BenchEvent::Event1 => "Event1".to_string(),
            BenchEvent::Event2 => "Event2".to_string(),
            BenchEvent::Event3 => "Event3".to_string(),
        }
    }
}

// Simple background task for benchmarking
struct BenchBackgroundTask {
    name: String,
    work_amount: usize,
}

impl BenchBackgroundTask {
    fn new(name: &str, work_amount: usize) -> Self {
        Self {
            name: name.to_string(),
            work_amount,
        }
    }
}

#[async_trait]
impl Executable for BenchBackgroundTask {
    fn name(&self) -> &str {
        &self.name
    }

    async fn initialize(&mut self) -> Result<(), OrchestratorError> {
        // Simulate initialization work
        for _ in 0..self.work_amount {
            black_box(());
        }
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), OrchestratorError> {
        // Simulate shutdown work
        for _ in 0..self.work_amount {
            black_box(());
        }
        Ok(())
    }
}

#[async_trait]
impl BackgroundTask for BenchBackgroundTask {
    async fn execute(&mut self) -> Result<(), OrchestratorError> {
        // Simulate execution work
        for _ in 0..self.work_amount {
            black_box(());
        }
        Ok(())
    }
}

// Simple event task for benchmarking
struct BenchEventTask {
    name: String,
    event: BenchEvent,
    work_amount: usize,
}

impl BenchEventTask {
    fn new(name: &str, event: BenchEvent, work_amount: usize) -> Self {
        Self {
            name: name.to_string(),
            event,
            work_amount,
        }
    }
}

#[async_trait]
impl Executable for BenchEventTask {
    fn name(&self) -> &str {
        &self.name
    }

    async fn initialize(&mut self) -> Result<(), OrchestratorError> {
        // Simulate initialization work
        for _ in 0..self.work_amount {
            black_box(());
        }
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), OrchestratorError> {
        // Simulate shutdown work
        for _ in 0..self.work_amount {
            black_box(());
        }
        Ok(())
    }
}

#[async_trait]
impl EventTask<BenchEvent> for BenchEventTask {
    fn subscribed_event(&self) -> &BenchEvent {
        &self.event
    }

    async fn handle_event(&mut self, _event: String) -> Result<(), OrchestratorError> {
        // Simulate event handling work
        for _ in 0..self.work_amount {
            black_box(());
        }
        Ok(())
    }
}

// Create event bus for benchmarking
fn create_bench_event_bus() -> EventBus<BenchEvent> {
    let configs = vec![
        (BenchEvent::Event1, ChannelConfig::new(10, "Bench Channel 1".to_string())),
        (BenchEvent::Event2, ChannelConfig::new(10, "Bench Channel 2".to_string())),
        (BenchEvent::Event3, ChannelConfig::new(10, "Bench Channel 3".to_string())),
    ];
    EventBus::new(configs)
}

fn bench_scheduler_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("scheduler_creation");
    
    group.bench_function("new", |b| {
        b.iter(|| {
            let event_bus = create_bench_event_bus();
            black_box(Scheduler::<BenchEvent>::new(event_bus));
        });
    });
    
    group.finish();
}

fn bench_task_registration(c: &mut Criterion) {
    let mut group = c.benchmark_group("task_registration");
    
    group.bench_function("register_background_task", |b| {
        b.iter(|| {
            let rt = Runtime::new().unwrap();
            rt.block_on(async {
                let event_bus = create_bench_event_bus();
                let mut scheduler = Scheduler::<BenchEvent>::new(event_bus);
                let task = Arc::new(Mutex::new(BenchBackgroundTask::new("Bench Background Task", 10)));
                black_box(scheduler.register_background_task(task));
            });
        });
    });
    
    group.bench_function("register_event_task", |b| {
        b.iter(|| {
            let rt = Runtime::new().unwrap();
            rt.block_on(async {
                let event_bus = create_bench_event_bus();
                let mut scheduler = Scheduler::<BenchEvent>::new(event_bus);
                let task = Arc::new(Mutex::new(BenchEventTask::new("Bench Event Task", BenchEvent::Event1, 10)));
                black_box(scheduler.register_event_task(task));
            });
        });
    });
    
    group.finish();
}

fn bench_task_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("task_scaling");
    
    for task_count in [1, 10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::new("background_tasks", task_count), task_count, |b, &count| {
            b.iter(|| {
                let rt = Runtime::new().unwrap();
                rt.block_on(async {
                    let event_bus = create_bench_event_bus();
                    let mut scheduler = Scheduler::<BenchEvent>::new(event_bus);
                    
                    for i in 0..count {
                        let task = Arc::new(Mutex::new(BenchBackgroundTask::new(&format!("Bench Background Task {}", i), 1)));
                        scheduler.register_background_task(task);
                    }
                    
                    black_box(scheduler)
                });
            });
        });
        
        group.bench_with_input(BenchmarkId::new("event_tasks", task_count), task_count, |b, &count| {
            b.iter(|| {
                let rt = Runtime::new().unwrap();
                rt.block_on(async {
                    let event_bus = create_bench_event_bus();
                    let mut scheduler = Scheduler::<BenchEvent>::new(event_bus);
                    
                    for i in 0..count {
                        let event = match i % 3 {
                            0 => BenchEvent::Event1,
                            1 => BenchEvent::Event2,
                            _ => BenchEvent::Event3,
                        };
                        let task = Arc::new(Mutex::new(BenchEventTask::new(&format!("Bench Event Task {}", i), event, 1)));
                        scheduler.register_event_task(task);
                    }
                    
                    black_box(scheduler)
                });
            });
        });
    }
    
    group.finish();
}

fn bench_start_shutdown(c: &mut Criterion) {
    let mut group = c.benchmark_group("start_shutdown");
    
    for task_count in [1, 5, 10].iter() {
        group.bench_with_input(BenchmarkId::new("start", task_count), task_count, |b, &count| {
            b.iter(|| {
                let rt = Runtime::new().unwrap();
                rt.block_on(async {
                    let event_bus = create_bench_event_bus();
                    let mut scheduler = Scheduler::<BenchEvent>::new(event_bus);
                    
                    // Register background tasks
                    for i in 0..count {
                        let task = Arc::new(Mutex::new(BenchBackgroundTask::new(&format!("Bench Background Task {}", i), 1)));
                        scheduler.register_background_task(task);
                    }
                    
                    // Register event tasks
                    for i in 0..count {
                        let event = match i % 3 {
                            0 => BenchEvent::Event1,
                            1 => BenchEvent::Event2,
                            _ => BenchEvent::Event3,
                        };
                        let task = Arc::new(Mutex::new(BenchEventTask::new(&format!("Bench Event Task {}", i), event, 1)));
                        scheduler.register_event_task(task);
                    }
                    
                    // Benchmark start operation
                    black_box(scheduler.start().await).unwrap();
                    
                    // Clean up - call shutdown to terminate all tasks
                    let _ = scheduler.shutdown().await;
                });
            });
        });
        
        group.bench_with_input(BenchmarkId::new("shutdown", task_count), task_count, |b, &count| {
            b.iter(|| {
                let rt = Runtime::new().unwrap();
                rt.block_on(async {
                    let event_bus = create_bench_event_bus();
                    let mut scheduler = Scheduler::<BenchEvent>::new(event_bus);
                    
                    // Register background tasks
                    for i in 0..count {
                        let task = Arc::new(Mutex::new(BenchBackgroundTask::new(&format!("Bench Background Task {}", i), 1)));
                        scheduler.register_background_task(task);
                    }
                    
                    // Register event tasks
                    for i in 0..count {
                        let event = match i % 3 {
                            0 => BenchEvent::Event1,
                            1 => BenchEvent::Event2,
                            _ => BenchEvent::Event3,
                        };
                        let task = Arc::new(Mutex::new(BenchEventTask::new(&format!("Bench Event Task {}", i), event, 1)));
                        scheduler.register_event_task(task);
                    }
                    
                    // Start the scheduler first (not part of the benchmark)
                    let _ = scheduler.start().await;
                    
                    // Benchmark shutdown operation
                    black_box(scheduler.shutdown().await).unwrap();
                });
            });
        });
    }
    
    group.finish();
}

fn bench_event_propagation(c: &mut Criterion) {
    let mut group = c.benchmark_group("event_propagation");
    
    for subscriber_count in [1, 5, 10].iter() {
        group.bench_with_input(BenchmarkId::new("subscribers", subscriber_count), subscriber_count, |b, &count| {
            b.iter(|| {
                let rt = Runtime::new().unwrap();
                rt.block_on(async {
                    let event_bus = create_bench_event_bus();
                    let mut scheduler = Scheduler::<BenchEvent>::new(event_bus);
                    
                    // Register event tasks, all subscribing to the same event
                    for i in 0..count {
                        let task = Arc::new(Mutex::new(BenchEventTask::new(&format!("Bench Event Task {}", i), BenchEvent::Event1, 1)));
                        scheduler.register_event_task(task);
                    }
                    
                    // Start the scheduler
                    let _ = scheduler.start().await;
                    
                    // Get a sender for the event
                    let sender = scheduler.event_bus().clone_sender(&BenchEvent::Event1).unwrap();
                    
                    // Benchmark sending the event (this will propagate to all subscribers)
                    black_box(sender.send("bench_event_data".to_string())).unwrap();
                    
                    // Allow a little time for events to be processed
                    tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
                    
                    // Clean up
                    let _ = scheduler.shutdown().await;
                });
            });
        });
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_scheduler_creation,
    bench_task_registration,
    bench_task_scaling,
    bench_start_shutdown,
    bench_event_propagation
);
criterion_main!(benches);