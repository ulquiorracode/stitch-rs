use machine_core::prelude::*;
use stitch_async::machine_adapter::{MachineTerminal, PipelineStepMachine};
use stitch_async::prelude::*;
use stitch_core::flow::FlowControl;
use stitch_core::middleware::Layer;
use stitch_core::pipeline::Pipeline;

#[repr(C, align(64))]
struct TestBlackboard {
    pub counter: u64,
}

impl Blackboard for TestBlackboard {}

// 1. Simple FSM to run inside MachineTerminal
enum CounterState {
    Active,
    Done,
}

struct CounterFsm {
    state: CounterState,
}

impl CounterFsm {
    const fn new() -> Self {
        Self {
            state: CounterState::Active,
        }
    }
}

impl Machine for CounterFsm {
    type Yield = &'static str;
    type Resume = u64;
    type Return = u64;

    fn step(&mut self, resume: Self::Resume) -> Step<Self::Yield, Self::Return> {
        match self.state {
            CounterState::Active => {
                if resume >= 100 {
                    self.state = CounterState::Done;
                    Step::Done(resume * 2)
                } else {
                    Step::Yielded("Value below 100")
                }
            }
            CounterState::Done => Step::Done(resume),
        }
    }
}

struct DoubleLayer;

impl AsyncLayer<TestBlackboard, u64, u64, &'static str> for DoubleLayer {
    async fn on_enter(
        &self,
        _ctx: &mut TestBlackboard,
        intent: u64,
    ) -> FlowControl<u64, u64, &'static str> {
        FlowControl::Proceed(intent)
    }

    async fn on_exit(&self, _ctx: &mut TestBlackboard, outcome: &mut Result<u64, &'static str>) {
        if let Ok(val) = outcome {
            *val += 1;
        }
    }
}

#[tokio::test]
async fn test_machine_terminal_in_async_pipeline() {
    let mut ctx = TestBlackboard { counter: 0 };
    let terminal = MachineTerminal::new(CounterFsm::new());

    let mut pipe = AsyncPipeline::on_terminal(terminal).wrap(DoubleLayer);

    // Yielded -> Err
    let res_err = pipe.dispatch(&mut ctx, 50).await;
    assert_eq!(res_err, Err("Value below 100"));

    // Done -> Ok with on_exit modification (+1)
    let res_ok = pipe.dispatch(&mut ctx, 150).await;
    assert_eq!(res_ok, Ok(301)); // (150 * 2) + 1
}

// 2. Test PipelineStepMachine wrapping an SMA Pipeline
struct SyncTerminal;

impl stitch_core::middleware::Terminal<TestBlackboard, u64, u64, &'static str> for SyncTerminal {
    fn execute(&mut self, ctx: &mut TestBlackboard, intent: u64) -> Result<u64, &'static str> {
        ctx.counter += intent;
        Ok(ctx.counter)
    }
}

struct SyncVerifyLayer;

impl Layer<TestBlackboard, u64, u64, &'static str> for SyncVerifyLayer {
    fn on_enter(
        &self,
        _ctx: &mut TestBlackboard,
        intent: u64,
    ) -> FlowControl<u64, u64, &'static str> {
        if intent == 0 {
            FlowControl::Halt("Intent cannot be zero")
        } else {
            FlowControl::Proceed(intent)
        }
    }

    fn on_exit(&self, _ctx: &mut TestBlackboard, _outcome: &mut Result<u64, &'static str>) {}
}

#[test]
fn test_pipeline_step_machine_execution() {
    let ctx = TestBlackboard { counter: 10 };
    let pipe = Pipeline::on_terminal(SyncTerminal).wrap(SyncVerifyLayer);

    let mut step_machine = PipelineStepMachine::new(pipe, ctx);

    // Error yields as TErr
    let err_step = step_machine.step(0);
    assert_eq!(err_step, Step::Yielded("Intent cannot be zero"));

    // Success finishes with Step::Done(outcome)
    let ok_step = step_machine.step(25);
    assert_eq!(ok_step, Step::Done(35));
    assert_eq!(step_machine.context().counter, 35);
}
