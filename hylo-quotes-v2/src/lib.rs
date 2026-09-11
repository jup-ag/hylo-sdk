//! Carved quote computation for the Hylo protocol (v2), adapted for the
//! Jupiter solana-3 stack. Retains protocol-state math and token operations;
//! RPC/client/simulation strategies are intentionally dropped.

use anyhow::{anyhow, Result};
use fix::prelude::{UFix64, N6, N9};
use hylo_idl::tokens::{
  StakePool, TokenMint, CBBTC, HYLOSOL, HYPE, JITOSOL, XBTC, XHYPE,
};

pub mod protocol_state;
pub mod token_operation;
pub mod util;

pub use protocol_state::{ProtocolState, UsdcExchangeState};

/// Marker trait for supported LST tokens.
pub trait LST: StakePool {}
impl LST for JITOSOL {}
impl LST for HYLOSOL {}

/// Exogenous collateral backing an exo pair, paired with its levercoin.
///
/// The exchange holds exo collateral as `N9` regardless of the collateral's
/// own decimals, so every route has to normalize on the way in and denormalize
/// on the way out. `to_n9`/`from_n9` keep that scaling in concrete-typed code
/// per collateral rather than in a generic `checked_convert`: cbBTC is `N8` and
/// must scale by 10, HYPE is already `N9` and must not. Getting that wrong
/// converts cleanly and misprices by 10x without erroring.
pub trait Exo: TokenMint {
  /// Levercoin minted against this collateral. Every Hylo levercoin is `N6`,
  /// which the bound pins so the exo routes can stay concrete about it.
  type Levercoin: TokenMint<Exp = N6>;

  /// Normalizes a collateral amount to the exchange's `N9` basis.
  ///
  /// # Errors
  /// * Overflow widening to `N9`
  fn to_n9(amount: UFix64<Self::Exp>) -> Result<UFix64<N9>>;

  /// Denormalizes an `N9` exchange amount back to collateral decimals.
  ///
  /// # Errors
  /// * Overflow narrowing from `N9`
  fn from_n9(amount: UFix64<N9>) -> Result<UFix64<Self::Exp>>;
}

impl Exo for CBBTC {
  type Levercoin = XBTC;

  fn to_n9(amount: UFix64<Self::Exp>) -> Result<UFix64<N9>> {
    amount
      .checked_convert()
      .ok_or_else(|| anyhow!("cbBTC N8->N9 overflow"))
  }

  fn from_n9(amount: UFix64<N9>) -> Result<UFix64<Self::Exp>> {
    amount
      .checked_convert()
      .ok_or_else(|| anyhow!("cbBTC N9->N8 overflow"))
  }
}

impl Exo for HYPE {
  type Levercoin = XHYPE;

  // HYPE is already N9: both directions are the identity.
  fn to_n9(amount: UFix64<Self::Exp>) -> Result<UFix64<N9>> {
    Ok(amount)
  }

  fn from_n9(amount: UFix64<N9>) -> Result<UFix64<Self::Exp>> {
    Ok(amount)
  }
}

/// Local marker allowing use of [`LST`]-adjacent types in trait-bound position
/// within this crate.
pub(crate) trait Local {}
impl Local for JITOSOL {}
impl Local for HYLOSOL {}
