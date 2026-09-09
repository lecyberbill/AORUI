// [WFGY] Zone: TRANSIT | λ: 0.2 | Fallbacks: 0 | Action: Declarative shared protocol definition (human UI first, optional agent control)
//! Declarative protocol shared between the Render Thread (winit/wgpu) and
//! application logic — whether driven by human interactions (mouse/keyboard)
//! or optionally by the asynchronous `agent-runtime`.
//!
//! Structural Invariant (INV-CORE-1): this crate must depend neither on `wgpu`,
//! nor on `winit`, nor on an active `tokio` runtime. It only defines pure data
//! types (events, patches, GPU memory-aligned layouts).
//!
//! INV-SEC-1 (Trust boundary): [`UiEvent`] and [`UiPatch`] derive
//! `Serialize`/`Deserialize` for internal usage (logging, replay, testing) —
//! currently transmitted over in-process channels (`tokio::sync::mpsc`,
//! `crossbeam_channel`). If a future extension exposes them over external
//! transports (IPC, network, shared file), any deserialization must explicitly
//! validate the source before use: never trust a `UiEvent`/`UiPatch` deserialized
//! from an untrusted origin.

pub mod events;
pub mod gpu_types;
pub mod patches;

pub use events::UiEvent;
pub use gpu_types::GpuSdfInstance;
pub use patches::{StepStatus, UiPatch};
