//! Outbox pattern abstractions for zero-allocation asynchronous and event-driven egress.

/// A stack-allocated, fixed-capacity ring buffer for staging domain events
/// produced during synchronous U-cycle traversal without heap allocation.
#[derive(Debug)]
pub struct Outbox<TEvent, const CAP: usize> {
    storage: [Option<TEvent>; CAP],
    len: usize,
}

impl<TEvent, const CAP: usize> Outbox<TEvent, CAP> {
    /// Creates a new empty outbox buffer.
    #[inline(always)]
    pub const fn new() -> Self {
        const {
            assert!(CAP > 0, "Outbox capacity must be greater than zero");
        }
        // In no_std const context, Option None array initialization
        Self {
            storage: [const { None }; CAP],
            len: 0,
        }
    }

    /// Number of events currently staged in the outbox.
    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Whether the outbox is empty.
    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Whether the outbox has reached maximum capacity.
    #[inline(always)]
    pub const fn is_full(&self) -> bool {
        self.len == CAP
    }

    /// Stages a domain event into the outbox buffer.
    ///
    /// Returns `Err(event)` if capacity is exhausted, preventing silent event loss.
    #[inline(always)]
    pub fn push(&mut self, event: TEvent) -> Result<(), TEvent> {
        if self.len < CAP {
            self.storage[self.len] = Some(event);
            self.len += 1;
            Ok(())
        } else {
            Err(event)
        }
    }

    /// Drains all staged events, passing each into the provided consumer closure.
    ///
    /// Leaves the outbox empty and ready for the next tick cycle.
    #[inline(always)]
    pub fn drain<F: FnMut(TEvent)>(&mut self, mut consumer: F) {
        for slot in self.storage.iter_mut().take(self.len) {
            if let Some(event) = slot.take() {
                consumer(event);
            }
        }
        self.len = 0;
    }
}

impl<TEvent, const CAP: usize> Default for Outbox<TEvent, CAP> {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    enum ServerEvent {
        PlayerSpawned(u32),
        PlayerKilled(u32),
    }

    #[test]
    fn test_outbox_push_and_drain() {
        let mut outbox = Outbox::<ServerEvent, 4>::new();
        assert!(outbox.is_empty());

        assert_eq!(outbox.push(ServerEvent::PlayerSpawned(1)), Ok(()));
        assert_eq!(outbox.push(ServerEvent::PlayerKilled(1)), Ok(()));
        assert_eq!(outbox.len(), 2);

        let mut collected = Vec::new();
        outbox.drain(|event| collected.push(event));

        assert_eq!(collected.len(), 2);
        assert_eq!(collected[0], ServerEvent::PlayerSpawned(1));
        assert_eq!(collected[1], ServerEvent::PlayerKilled(1));
        assert!(outbox.is_empty());
    }

    #[test]
    fn test_outbox_capacity_rejection() {
        let mut outbox = Outbox::<u32, 2>::new();
        assert_eq!(outbox.push(1), Ok(()));
        assert_eq!(outbox.push(2), Ok(()));
        assert!(outbox.is_full());

        // Must reject 3rd without panicking or dropping silently
        let overflow = outbox.push(3);
        assert_eq!(overflow, Err(3));
        assert_eq!(outbox.len(), 2);
    }
}
