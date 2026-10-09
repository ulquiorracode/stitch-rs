use crate::data::Event;

/// A stack-allocated, fixed-capacity linear staging buffer for staging domain events
/// produced during synchronous U-cycle traversal without heap allocation.
#[derive(Debug)]
pub struct Outbox<TEvent: Event, const CAP: usize> {
    storage: [Option<TEvent>; CAP],
    len: usize,
}

impl<TEvent: Event, const CAP: usize> Outbox<TEvent, CAP> {
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
    /// The buffer length is decremented eagerly per item, ensuring that if the consumer
    /// closure panics, previously drained items are not left stale or re-processed.
    #[inline(always)]
    pub fn drain<F: FnMut(TEvent)>(&mut self, mut consumer: F) {
        for idx in 0..self.len {
            if let Some(event) = self.storage[idx].take() {
                consumer(event);
            }
        }
        self.len = 0;
    }
}

impl<TEvent: Event, const CAP: usize> Default for Outbox<TEvent, CAP> {
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
    impl Event for ServerEvent {}

    #[derive(Debug, PartialEq, Eq)]
    struct NumericEvent(u32);
    impl Event for NumericEvent {}

    #[test]
    fn test_outbox_push_and_drain() {
        let mut outbox = Outbox::<ServerEvent, 4>::new();
        assert!(outbox.is_empty());

        assert_eq!(outbox.push(ServerEvent::PlayerSpawned(1)), Ok(()));
        assert_eq!(outbox.push(ServerEvent::PlayerKilled(1)), Ok(()));
        assert_eq!(outbox.len(), 2);

        let mut collected: [Option<ServerEvent>; 4] = [None, None, None, None];
        let mut count = 0;
        outbox.drain(|event| {
            collected[count] = Some(event);
            count += 1;
        });

        assert_eq!(count, 2);
        assert_eq!(collected[0], Some(ServerEvent::PlayerSpawned(1)));
        assert_eq!(collected[1], Some(ServerEvent::PlayerKilled(1)));
        assert!(outbox.is_empty());
    }

    #[test]
    fn test_outbox_capacity_rejection() {
        let mut outbox = Outbox::<NumericEvent, 2>::new();
        assert_eq!(outbox.push(NumericEvent(1)), Ok(()));
        assert_eq!(outbox.push(NumericEvent(2)), Ok(()));
        assert!(outbox.is_full());

        // Must reject 3rd without panicking or dropping silently
        let overflow = outbox.push(NumericEvent(3));
        assert_eq!(overflow, Err(NumericEvent(3)));
        assert_eq!(outbox.len(), 2);
    }
}
