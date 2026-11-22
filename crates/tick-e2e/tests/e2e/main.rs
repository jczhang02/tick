//! E2E tests: the real app in headless mutter, driven through AT-SPI and real input.
//!
//! All E2E tests live in this one binary so they share the `TICK_E2E_JOBS` semaphore.
//! Run with `just e2e` (it builds the `tick` binary and the harness fixture first).

mod fixture;
mod harness;
mod screenshots;

use std::time::Duration;

/// Default wait for UI changes. Generous because CI renders in software.
pub const WAIT: Duration = Duration::from_secs(10);
