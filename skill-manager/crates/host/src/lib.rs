//! 本机 Host：咨询锁、运行时、HTTP、确认计划与内置管理 Agent。
//! 技能业务仍在 `skill-manager-core`；本 crate 不重写库规则。

pub mod agent;
pub mod commands;
pub mod events;
pub mod http;
pub mod identity;
pub mod identity_store;
pub mod invoke;
pub mod lock;
pub mod pair;
pub mod plans;
pub mod runtime;
pub mod serve;
pub mod service;

pub use events::{BroadcastSink, EventSink, FanoutSink, NoopSink};
pub use invoke::{InvokeError, InvokeSource};
pub use lock::{HostLock, LockError, LockMeta};
pub use runtime::{HostRuntime, HostStatus};
