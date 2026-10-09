//! Example 01: Authentication & Logging U-Cycle
//!
//! Demonstrates the two-phase U-cycle traversal:
//! - Descent: AuthLayer verifies permissions; proceeds to Terminal.
//! - Point of Puncture: Terminal executes work against material context.
//! - Ascent: LoggingLayer audits execution telemetry on return.

use stitch_rs::prelude::*;

#[blackboard]
#[repr(C, align(64))]
struct RequestContext {
    user_id: u64,
    is_admin: bool,
    log_count: usize,
}

impl Blackboard for RequestContext {}

struct AuthLayer;

impl Layer<RequestContext, &'static str, &'static str, &'static str> for AuthLayer {
    fn on_enter(
        &self,
        ctx: &mut RequestContext,
        intent: &'static str,
    ) -> FlowControl<&'static str, &'static str, &'static str> {
        if intent == "admin_action" && !ctx.is_admin {
            FlowControl::Halt("Unauthorized: Administrator privileges required")
        } else {
            FlowControl::Proceed(intent)
        }
    }

    fn on_exit(
        &self,
        _ctx: &mut RequestContext,
        _outcome: &mut Result<&'static str, &'static str>,
    ) {
    }
}

struct LoggingLayer;

impl Layer<RequestContext, &'static str, &'static str, &'static str> for LoggingLayer {
    fn on_enter(
        &self,
        _ctx: &mut RequestContext,
        intent: &'static str,
    ) -> FlowControl<&'static str, &'static str, &'static str> {
        FlowControl::Proceed(intent)
    }

    fn on_exit(&self, ctx: &mut RequestContext, outcome: &mut Result<&'static str, &'static str>) {
        ctx.log_count += 1;
        match outcome {
            Ok(msg) => println!("[AUDIT] Operation succeeded: {msg}"),
            Err(err) => println!("[AUDIT] Operation failed: {err}"),
        }
    }
}

struct ActionTerminal;

impl Terminal<RequestContext, &'static str, &'static str, &'static str> for ActionTerminal {
    fn execute(
        &mut self,
        _ctx: &mut RequestContext,
        intent: &'static str,
    ) -> Result<&'static str, &'static str> {
        if intent == "admin_action" {
            Ok("Admin action executed successfully")
        } else {
            Ok("Standard action executed")
        }
    }
}

fn main() {
    let mut pipeline = Pipeline::on_terminal(ActionTerminal)
        .wrap(AuthLayer)
        .wrap(LoggingLayer);

    let mut user_ctx = RequestContext {
        user_id: 1001,
        is_admin: false,
        log_count: 0,
    };

    println!("--- Test 1: Standard Action ---");
    let res = pipeline.dispatch(&mut user_ctx, "standard_action");
    assert_eq!(res, Ok("Standard action executed"));
    assert_eq!(user_ctx.log_count, 1);

    println!("\n--- Test 2: Unauthorized Admin Action ---");
    let err = pipeline.dispatch(&mut user_ctx, "admin_action");
    assert!(err.is_err());
    assert_eq!(user_ctx.log_count, 2);

    println!("\n--- Test 3: Authorized Admin Action ---");
    user_ctx.is_admin = true;
    let ok = pipeline.dispatch(&mut user_ctx, "admin_action");
    assert_eq!(ok, Ok("Admin action executed successfully"));
    assert_eq!(user_ctx.log_count, 3);

    println!("\nAll U-cycle checks passed cleanly.");
}
