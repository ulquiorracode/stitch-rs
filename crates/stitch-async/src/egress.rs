//! Asynchronous Outbox Streaming Egress Worker.
//!
//! Bridges the ultra-low-latency synchronous game/core loop with asynchronous network sinks.
//! During the synchronous U-cycle, events are collected with zero heap allocation into a fixed-capacity
//! [`stitch_core::Outbox`]. An asynchronous background worker drains these batches and writes them
//! into async sinks (TCP sockets, WebSockets, gRPC, Kafka, file WAL) without stalling the simulation tick.

use core::future::Future;
use core::mem::MaybeUninit;
use stitch_core::data::Event;
use stitch_core::outbox::Outbox;

/// An asynchronous destination sink for drained domain events.
pub trait OutboxEgressSink<TEvent: Event>: Send + Sync {
    /// Error type produced on write failure.
    type Error: Send;

    /// Sends a batch of events asynchronously to the destination transport.
    fn send_batch(
        &mut self,
        batch: &[TEvent],
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
}

/// Background worker managing asynchronous draining and egress transmission of staged outbox events.
pub struct OutboxEgressWorker<TEvent: Event, TSink: OutboxEgressSink<TEvent>, const CAP: usize> {
    sink: TSink,
    staging_buffer: [MaybeUninit<TEvent>; CAP],
    staging_len: usize,
}

impl<TEvent: Event, TSink: OutboxEgressSink<TEvent>, const CAP: usize>
    OutboxEgressWorker<TEvent, TSink, CAP>
{
    /// Creates a new egress worker wrapping the target sink.
    pub const fn new(sink: TSink) -> Self {
        Self {
            sink,
            // SAFETY: An uninitialized array of MaybeUninit is valid initialization in Rust.
            staging_buffer: [const { MaybeUninit::uninit() }; CAP],
            staging_len: 0,
        }
    }

    /// Accesses a mutable reference to the underlying sink.
    pub fn sink_mut(&mut self) -> &mut TSink {
        &mut self.sink
    }

    /// Borrows the underlying sink.
    pub fn sink(&self) -> &TSink {
        &self.sink
    }

    /// Ingests all staged events from a synchronous [`Outbox`] into worker's staging buffer,
    /// and flushes them to the async sink.
    ///
    /// Transmits a contiguous `&[TEvent]` slice with zero dynamic allocations.
    pub async fn ingest_and_flush(
        &mut self,
        outbox: &mut Outbox<TEvent, CAP>,
    ) -> Result<usize, TSink::Error> {
        if outbox.is_empty() {
            return Ok(0);
        }

        self.staging_len = 0;
        outbox.drain(|event| {
            if self.staging_len < CAP {
                self.staging_buffer[self.staging_len].write(event);
                self.staging_len += 1;
            }
        });

        let count = self.staging_len;
        if count == 0 {
            return Ok(0);
        }

        // SAFETY:
        // 1. `staging_buffer[0..count]` has been fully initialized by `outbox.drain` write calls above.
        // 2. `MaybeUninit<TEvent>` has the exact same layout and alignment as `TEvent`.
        // 3. The slice borrow is strictly limited to `count` elements.
        let slice: &[TEvent] = unsafe {
            core::slice::from_raw_parts(self.staging_buffer.as_ptr() as *const TEvent, count)
        };

        let send_res = self.sink.send_batch(slice).await;

        // Cleanup: drop all initialized elements to prevent resource leak
        for i in 0..count {
            // SAFETY: Elements 0..count were initialized.
            unsafe {
                self.staging_buffer[i].assume_init_drop();
            }
        }
        self.staging_len = 0;

        send_res?;

        Ok(count)
    }
}

impl<TEvent: Event, TSink: OutboxEgressSink<TEvent>, const CAP: usize> Drop
    for OutboxEgressWorker<TEvent, TSink, CAP>
{
    fn drop(&mut self) {
        for i in 0..self.staging_len {
            // SAFETY: Elements 0..staging_len are valid initialized instances.
            unsafe {
                self.staging_buffer[i].assume_init_drop();
            }
        }
    }
}
