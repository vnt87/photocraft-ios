//! Apple Pencil input (ticket MAC-4): pressure, tilt, barrel roll, hover and double-tap.
//!
//! - [`mapping`]: pure UIKit-units -> `PenSample` conversion (host-tested).
//! - [`bridge`]: pure state machine between UIKit callbacks and PhotoCraft's `StylusFeed`, the
//!   hover pointer and the double-tap (host-tested). Its module docs explain what winit already
//!   delivers and why only the rest is bridged.
//! - `native` (iOS only): the audited UIKit boundary that reads `UITouch`,
//!   `UIHoverGestureRecognizer` and `UIPencilInteraction` and calls into the bridge.

pub mod bridge;
pub mod mapping;

#[cfg(target_os = "ios")]
#[allow(unsafe_code)]
mod native;
#[cfg(target_os = "ios")]
pub use native::install;
