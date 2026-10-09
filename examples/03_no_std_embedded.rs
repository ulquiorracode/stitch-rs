//! Example 03: Embedded & #![no_std] Bare-Metal Execution
//!
//! Demonstrates the Scrooge systems invariants in zero-allocation environments:
//! - `#![no_std]` compliance without heap allocations.
//! - 64-byte L1D cache-line aligned blackboard contexts.
//! - Transparent register identity tokens and IDs.
//! - Static monomorphic dispatch with zero runtime dynamic dispatch (`dyn`).

#![no_std]

use stitch_rs::prelude::*;

#[blackboard]
#[repr(C, align(64))]
struct DeviceBlackboard {
    pub sensor_raw: u32,
    pub filtered_value: u32,
    pub status_code: u8,
}

impl Blackboard for DeviceBlackboard {}

#[token]
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
struct SensorToken(u64);

impl StitchToken for SensorToken {
    fn raw_u64(&self) -> u64 {
        self.0
    }
}

#[id]
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
struct SensorId(SensorToken);

impl StitchId for SensorId {
    type Token = SensorToken;
    fn token(&self) -> Self::Token {
        self.0
    }
}

struct ThresholdFilterLayer {
    min_threshold: u32,
}

impl Layer<DeviceBlackboard, u32, u32, u8> for ThresholdFilterLayer {
    fn on_enter(&self, _ctx: &mut DeviceBlackboard, raw_reading: u32) -> FlowControl<u32, u32, u8> {
        if raw_reading < self.min_threshold {
            // Signal error code 1: Below threshold
            FlowControl::Halt(1)
        } else {
            FlowControl::Proceed(raw_reading)
        }
    }

    fn on_exit(&self, ctx: &mut DeviceBlackboard, outcome: &mut Result<u32, u8>) {
        match outcome {
            Ok(val) => {
                ctx.filtered_value = *val;
                ctx.status_code = 0;
            }
            Err(code) => {
                ctx.status_code = *code;
            }
        }
    }
}

struct SensorOutputTerminal;

impl Terminal<DeviceBlackboard, u32, u32, u8> for SensorOutputTerminal {
    fn execute(&mut self, ctx: &mut DeviceBlackboard, reading: u32) -> Result<u32, u8> {
        ctx.sensor_raw = reading;
        // Apply fixed-point calibration
        Ok(reading * 2)
    }
}

fn run_pipeline() {
    let mut dev_ctx = DeviceBlackboard {
        sensor_raw: 0,
        filtered_value: 0,
        status_code: 0,
    };

    let mut pipeline = Pipeline::on_terminal(SensorOutputTerminal)
        .wrap(ThresholdFilterLayer { min_threshold: 10 });

    // 1. Valid sensor sample
    let res = pipeline.dispatch(&mut dev_ctx, 15);
    assert!(res == Ok(30));
    assert!(dev_ctx.sensor_raw == 15);
    assert!(dev_ctx.filtered_value == 30);
    assert!(dev_ctx.status_code == 0);

    // 2. Filtered sensor sample (Halt)
    let err = pipeline.dispatch(&mut dev_ctx, 5);
    assert!(err == Err(1));
    assert!(dev_ctx.status_code == 1);
}

// In standard test harness, call run_pipeline
fn main() {
    run_pipeline();
}
