#[derive(Debug, Clone)]
pub struct ChannelConfig {
    capacity: usize,
    #[allow(dead_code)]
    description: String,
}

impl ChannelConfig {
    pub fn new(capacity: usize, description: String) -> Self {
        Self {
            capacity,
            description,
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }
}
