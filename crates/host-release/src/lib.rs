//! 本仓发布配方的 Rust 实现；工作流投影继续归 chrono-ci。
#![allow(unexpected_cfgs)]

#[cfg(not(verus_keep_ghost))]
pub mod bootstrap;
mod codec;
#[cfg(not(verus_keep_ghost))]
pub mod committed;
pub mod delivery;
pub mod evidence;
pub mod execution;
mod files;
pub mod frontend;
pub mod identity;
pub mod local;
pub mod observed;
pub mod process;
pub mod publication;
pub mod receipt;
pub mod recipe;
mod registrations;
pub mod staging;
pub mod transport;
pub mod verified_recipe;
