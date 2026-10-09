use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use stitch_async::prelude::*;

#[repr(C, align(64))]
struct AsyncBlackboard {
    pub balance: u64,
    pub log_count: usize,
    pub log: [&'static str; 8],
}

impl Blackboard for AsyncBlackboard {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AccountIntent {
    Deposit(u64),
    Withdraw(u64),
    AuditPing,
}

struct AsyncAuthLayer {
    min_amount: u64,
}

impl AsyncLayer<AsyncBlackboard, AccountIntent, u64, &'static str> for AsyncAuthLayer {
    async fn on_enter(
        &self,
        _ctx: &mut AsyncBlackboard,
        intent: AccountIntent,
    ) -> FlowControl<AccountIntent, u64, &'static str> {
        match intent {
            AccountIntent::Withdraw(amt) if amt < self.min_amount => {
                FlowControl::Halt("Withdrawal amount below minimum threshold")
            }
            AccountIntent::AuditPing => FlowControl::ShortCircuit(777),
            _ => FlowControl::Proceed(intent),
        }
    }

    async fn on_exit(&self, ctx: &mut AsyncBlackboard, outcome: &mut Result<u64, &'static str>) {
        if ctx.log_count < ctx.log.len() {
            ctx.log[ctx.log_count] = match outcome {
                Ok(777) => "AuthLayer: ShortCircuit 777",
                Ok(_) => "AuthLayer: Success",
                Err(e) => *e,
            };
            ctx.log_count += 1;
        }
    }
}

struct AsyncRateLimitLayer;

impl AsyncLayer<AsyncBlackboard, AccountIntent, u64, &'static str> for AsyncRateLimitLayer {
    async fn on_enter(
        &self,
        _ctx: &mut AsyncBlackboard,
        intent: AccountIntent,
    ) -> FlowControl<AccountIntent, u64, &'static str> {
        FlowControl::Proceed(intent)
    }

    async fn on_exit(&self, ctx: &mut AsyncBlackboard, _outcome: &mut Result<u64, &'static str>) {
        if ctx.log_count < ctx.log.len() {
            ctx.log[ctx.log_count] = "RateLimit: Passed";
            ctx.log_count += 1;
        }
    }
}

struct AccountTerminal;

impl AsyncTerminal<AsyncBlackboard, AccountIntent, u64, &'static str> for AccountTerminal {
    async fn execute(
        &mut self,
        ctx: &mut AsyncBlackboard,
        intent: AccountIntent,
    ) -> Result<u64, &'static str> {
        match intent {
            AccountIntent::Deposit(amt) => {
                ctx.balance += amt;
                Ok(ctx.balance)
            }
            AccountIntent::Withdraw(amt) => {
                if ctx.balance >= amt {
                    ctx.balance -= amt;
                    Ok(ctx.balance)
                } else {
                    Err("Insufficient funds")
                }
            }
            AccountIntent::AuditPing => Ok(ctx.balance),
        }
    }
}

#[tokio::test]
async fn test_monomorphic_async_pipeline_u_cycle() {
    let mut ctx = AsyncBlackboard {
        balance: 100,
        log_count: 0,
        log: [""; 8],
    };

    let mut pipe = AsyncPipeline::on_terminal(AccountTerminal)
        .wrap(AsyncRateLimitLayer)
        .wrap(AsyncAuthLayer { min_amount: 10 });

    // 1. Success dispatch
    let res = pipe.dispatch(&mut ctx, AccountIntent::Deposit(50)).await;
    assert_eq!(res, Ok(150));
    assert_eq!(ctx.balance, 150);
    assert_eq!(ctx.log[0], "RateLimit: Passed");
    assert_eq!(ctx.log[1], "AuthLayer: Success");

    // 2. Short-circuit dispatch
    ctx.log_count = 0;
    let res_sc = pipe.dispatch(&mut ctx, AccountIntent::AuditPing).await;
    assert_eq!(res_sc, Ok(777));
    assert_eq!(ctx.balance, 150); // Terminal was bypassed
    assert_eq!(ctx.log[0], "AuthLayer: ShortCircuit 777");

    // 3. Halt dispatch
    ctx.log_count = 0;
    let res_halt = pipe.dispatch(&mut ctx, AccountIntent::Withdraw(5)).await;
    assert_eq!(res_halt, Err("Withdrawal amount below minimum threshold"));
    assert_eq!(ctx.balance, 150);
    assert_eq!(ctx.log[0], "Withdrawal amount below minimum threshold");
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AuditEvent {
    pub code: u32,
    pub description: &'static str,
}
impl Event for AuditEvent {}

struct MockNetworkSink {
    pub received_batches: Arc<AtomicU64>,
    pub total_events: Arc<AtomicU64>,
}

impl OutboxEgressSink<AuditEvent> for MockNetworkSink {
    type Error = &'static str;

    async fn send_batch(&mut self, batch: &[AuditEvent]) -> Result<(), Self::Error> {
        self.received_batches.fetch_add(1, Ordering::SeqCst);
        self.total_events
            .fetch_add(batch.len() as u64, Ordering::SeqCst);
        Ok(())
    }
}

#[tokio::test]
async fn test_outbox_egress_worker_streaming() {
    let batches = Arc::new(AtomicU64::new(0));
    let events = Arc::new(AtomicU64::new(0));

    let sink = MockNetworkSink {
        received_batches: Arc::clone(&batches),
        total_events: Arc::clone(&events),
    };

    let mut worker = OutboxEgressWorker::<AuditEvent, _, 16>::new(sink);
    let mut outbox = Outbox::<AuditEvent, 16>::new();

    // Stage events synchronously in game tick
    outbox
        .push(AuditEvent {
            code: 1,
            description: "PlayerConnect",
        })
        .unwrap();
    outbox
        .push(AuditEvent {
            code: 2,
            description: "PlayerSpawn",
        })
        .unwrap();
    outbox
        .push(AuditEvent {
            code: 3,
            description: "PlayerEquip",
        })
        .unwrap();
    assert_eq!(outbox.len(), 3);

    // Asynchronously drain and flush
    let count = worker.ingest_and_flush(&mut outbox).await.unwrap();
    assert_eq!(count, 3);
    assert!(outbox.is_empty());
    assert_eq!(batches.load(Ordering::SeqCst), 1);
    assert_eq!(events.load(Ordering::SeqCst), 3);

    // Empty flush returns 0 without calling sink
    let empty_count = worker.ingest_and_flush(&mut outbox).await.unwrap();
    assert_eq!(empty_count, 0);
    assert_eq!(batches.load(Ordering::SeqCst), 1);
}
