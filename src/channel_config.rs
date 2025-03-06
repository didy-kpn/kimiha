#[derive(Debug, Clone)]
pub struct ChannelConfig {
    capacity: usize,
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
