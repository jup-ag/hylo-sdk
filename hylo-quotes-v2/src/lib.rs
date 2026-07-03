//! Carved quote computation for the Hylo protocol (v2), adapted for the
//! Jupiter solana-3 stack. Retains protocol-state math and token operations;
//! RPC/client/simulation strategies are intentionally dropped.

use hylo_idl::tokens::{StakePool, HYLOSOL, JITOSOL};

pub mod protocol_state;
pub mod token_operation;
pub mod util;

pub use protocol_state::{ProtocolState, UsdcExchangeState};

/// Marker trait for supported LST tokens.
pub trait LST: StakePool {}
impl LST for JITOSOL {}
impl LST for HYLOSOL {}

/// Local marker allowing use of [`LST`]-adjacent types in trait-bound position
/// within this crate.
pub(crate) trait Local {}
impl Local for JITOSOL {}
impl Local for HYLOSOL {}
