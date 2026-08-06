use embassy_time::Instant;

pub struct LogTimeOfScope {
    start: Instant,
    name: &'static str,
}

impl LogTimeOfScope {
    pub fn new(name: &'static str) -> Self {
        Self {
            start: Instant::now(),
            name,
        }
    }
}

impl Drop for LogTimeOfScope {
    fn drop(&mut self) {
        let took = Instant::now() - self.start;
        defmt::info!("{} took {} ms", self.name, took.as_millis());
    }
}
