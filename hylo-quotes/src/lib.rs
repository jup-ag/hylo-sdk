//! Type-safe quote computation and transaction building for the Hylo protocol.
//!
//! Provides strategies for computing exchange rates, building Solana
//! instructions, and estimating compute units using either protocol state or
//! transaction simulation.
//!
//! # Strategies
//!
//! Two quote strategies are available:
//!
//! - **`ProtocolStateStrategy`**: Computes quotes using protocol state and SDK
//!   math. Fast and doesn't require transaction simulation, but doesn't check
//!   wallet balances.
//! - **`SimulationStrategy`**: Computes quotes by simulating transactions.
//!   Slower but validates that transactions would actually succeed (e.g.,
//!   checks wallet balances).
//!
//! # Examples
//!
//! ## Using `ProtocolStateStrategy`
//!
//! ```rust,ignore
//! use hylo_quotes::prelude::*;
//! use solana_rpc_client::nonblocking::rpc_client::RpcClient;
//! use std::sync::Arc;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let rpc_client = Arc::new(RpcClient::new_with_commitment(
//!   "https://api.mainnet-beta.solana.com".to_string(),
//!   CommitmentConfig::confirmed(),
//! ));
//! let state_provider = Arc::new(RpcStateProvider::new(rpc_client));
//!
//! let strategy = ProtocolStateStrategy::new(state_provider);
//!
//! let user = Pubkey::new_unique();
//! let amount_in = 1_000_000_000; // 1 JitoSOL (9 decimals)
//! let slippage_tolerance = 50; // 0.5%
//!
//! // Generates a tagged quote from runtime `Pubkeys`
//! let (quote, metadata) = strategy
//!   .runtime_quote_with_metadata(JITOSOL::MINT, HYUSD::MINT, amount_in, user, slippage_tolerance)
//!   .await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Using `SimulationStrategy`
//!
//! ```rust,ignore
//! use hylo_clients::prelude::*;
//! use hylo_quotes::prelude::*;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let router_client = RouterClient::new_random_keypair(
//!   Cluster::Mainnet,
//!   CommitmentConfig::confirmed(),
//! )?;
//!
//! let strategy = SimulationStrategy::new(router_client);
//!
//! let user = Pubkey::new_unique();
//! let amount_in = 1_000_000_000; // 1 JitoSOL (9 decimals)
//! let slippage_tolerance = 50; // 0.5%
//!
//! // SimulationStrategy validates balances, so if we get a quote, the transaction would succeed
//! let (quote, metadata) = strategy
//!   .runtime_quote_with_metadata(JITOSOL::MINT, HYUSD::MINT, amount_in, user, slippage_tolerance)
//!   .await?;
//!
//! # Ok(())
//! # }
//! ```
//!
//! ## Low-level output with `TokenOperationExt`
//!
//! For direct access to protocol math without transaction building, use
//! [`token_operation::TokenOperationExt`]. The `output` method provides
//! turbofish syntax for specifying token pairs:
//!
//! ```rust,ignore
//! use hylo_quotes::prelude::*;
//! use solana_rpc_client::nonblocking::rpc_client::RpcClient;
//! use std::sync::Arc;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let rpc_client = Arc::new(RpcClient::new("https://api.mainnet-beta.solana.com".into()));
//! let provider = RpcStateProvider::new(rpc_client);
//! let state = provider.fetch_state().await?;
//!
//! let amount_in = UFix64::new(1_000_000_000); // 1 JITOSOL
//! let output = state.output::<JITOSOL, HYUSD>(amount_in)?;
//! # Ok(())
//! # }
//! ```

// Jupiter fork: everything that builds transactions or reaches the chain
// through an RpcClient is gated behind `#[cfg(any())]`, which is never true.
// The sources stay on disk untouched so upstream merges keep applying, but
// only `protocol_state` and `token_operation` are compiled.
#[cfg(any())]
use anchor_client::solana_sdk::instruction::Instruction;
#[cfg(any())]
use anchor_lang::prelude::Pubkey;
#[cfg(any())]
use fix::prelude::{UFix64, UFixValue64};
#[cfg(any())]
use fix::typenum::Integer;
use hylo_idl::tokens::{
  StakePool, CBBTC, HYLOSOL, HYPE, JITOSOL, ONYC, PST, WETH, ZEC,
};
use hylo_idl::with_exo_pairs;

#[cfg(any())]
pub mod prelude;
pub mod protocol_state;
#[cfg(any())]
mod protocol_state_strategy;
#[cfg(any())]
mod quote_metadata;
#[cfg(any())]
mod quote_strategy;
#[cfg(any())]
mod runtime_quote_strategy;
#[cfg(any())]
pub mod simulated_operation;
#[cfg(any())]
mod simulation_strategy;
pub mod token_operation;

#[cfg(any())]
pub use hylo_clients::util::LST;
#[cfg(any())]
pub use protocol_state_strategy::ProtocolStateStrategy;
#[cfg(any())]
pub use quote_metadata::{Operation, QuoteMetadata};
#[cfg(any())]
pub use quote_strategy::QuoteStrategy;
#[cfg(any())]
pub use runtime_quote_strategy::RuntimeQuoteStrategy;
#[cfg(any())]
pub use simulated_operation::ComputeUnitInfo;
#[cfg(any())]
pub use simulation_strategy::SimulationStrategy;

/// Restates `hylo_clients::util::LST`, since hylo-clients is not built in the
/// Jupiter fork.
pub trait LST: StakePool {}
impl LST for JITOSOL {}
impl LST for HYLOSOL {}

/// Default buffered compute units for all exchange operations.
///
/// This is a buffered estimate (higher than measured values ~74k-97k CU) that
/// provides a safe default for all current quote operations. Measured values
/// came from calibration tool, but this value includes additional buffer for
/// safety across all operation types.
///
/// In the future, this could be replaced with per-instruction defaults based
/// on more comprehensive statistical analysis.
#[cfg(any())]
pub const DEFAULT_CUS_WITH_BUFFER: u64 = 100_000;

/// Typed executable quote with amounts, instructions, and compute units.
#[cfg(any())]
#[derive(Clone, Debug)]
pub struct ExecutableQuote<In: Integer, Out: Integer, Fee: Integer> {
  pub amount_in: UFix64<In>,
  pub amount_out: UFix64<Out>,
  pub compute_units: u64,
  pub compute_unit_strategy: ComputeUnitStrategy,
  pub fee_amount: UFix64<Fee>,
  pub fee_mint: Pubkey,
  pub instructions: Vec<Instruction>,
  pub address_lookup_tables: Vec<Pubkey>,
}

/// Executable quote with runtime exponent information.
#[cfg(any())]
#[derive(Clone, Debug)]
pub struct ExecutableQuoteValue {
  pub amount_in: UFixValue64,
  pub amount_out: UFixValue64,
  pub compute_units: u64,
  pub compute_unit_strategy: ComputeUnitStrategy,
  pub fee_amount: UFixValue64,
  pub fee_mint: Pubkey,
  pub instructions: Vec<Instruction>,
  pub address_lookup_tables: Vec<Pubkey>,
}

#[cfg(any())]
impl<In: Integer, Out: Integer, Fee: Integer>
  From<ExecutableQuote<In, Out, Fee>> for ExecutableQuoteValue
{
  fn from(quote: ExecutableQuote<In, Out, Fee>) -> ExecutableQuoteValue {
    ExecutableQuoteValue {
      amount_in: quote.amount_in.into(),
      amount_out: quote.amount_out.into(),
      compute_units: quote.compute_units,
      compute_unit_strategy: quote.compute_unit_strategy,
      fee_amount: quote.fee_amount.into(),
      fee_mint: quote.fee_mint,
      instructions: quote.instructions,
      address_lookup_tables: quote.address_lookup_tables,
    }
  }
}

#[cfg(any())]
#[derive(Clone, Debug)]
pub enum ComputeUnitStrategy {
  /// Estimated compute units based on historical data
  Estimated,
  /// Compute units returned from simulation results
  Simulated,
}

/// This crate builds on [`hylo_clients::util::LST`] in core traits like
/// [`QuoteStrategy<L, OUT>`].
///
/// The [`Local`] marker allows us to use [`LST`] in trait bound position while
/// telling the compiler that changes in `hylo-clients` won't affect local
/// impls.
pub(crate) trait Local {}
impl Local for JITOSOL {}
impl Local for HYLOSOL {}

pub(crate) trait LocalExo {}

macro_rules! impl_local_exo {
  ($(($exo:ident, $lever:ident, $exp:ty)),+ $(,)?) => {
    $(impl LocalExo for $exo {})+
  };
}

with_exo_pairs!(impl_local_exo);
