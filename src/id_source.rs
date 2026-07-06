pub(crate) struct IdSource {
    next: i64,
}

impl Default for IdSource {
    fn default() -> Self {
        Self { next: 1 }
    }
}

impl IdSource {
    pub(crate) fn next(&mut self) -> i64 {
        let next = self.next;
        self.next += 1;
        next
    }
}
