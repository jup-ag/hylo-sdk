//! Router-facing helpers for integrators.
//!
//! The router program picks the exchange or earn pool instruction from the
//! `(token_a, token_b)` pair and forwards the accounts it is given. This module
//! is the off-chain mirror of that dispatch: [`route_accounts`] resolves a pair
//! to the inner accounts, [`ProtocolState::quote_route`] quotes the same pair in
//! raw token atoms, and [`ROUTES`] lists every pair both functions know. Adding
//! a pair is one row in the table below.

use anchor_lang::prelude::{AccountMeta, Pubkey};
use anchor_lang::ToAccountMetas;
use anyhow::{anyhow, Result};
use fix::prelude::UFix64;
use hylo_core::pyth::PythOracle;
use hylo_core::solana_clock::SolanaClock;
use hylo_idl::earn_pool::account_builders as earn_pool;
use hylo_idl::exchange::account_builders as exchange;
use hylo_idl::tokens::{
  StakePool, TokenMint, CBBTC, HYLOSOL, HYPE, HYUSD, JITOSOL, SHYUSD, USDC,
  XBTC, XHYPE, XSOL,
};

use crate::protocol_state::ProtocolState;
use crate::token_operation::TokenOperationExt;

/// Quote for one router pair in raw token atoms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteQuote {
  pub in_amount: u64,
  pub out_amount: u64,
  pub fee_amount: u64,
  /// Amount the fee was charged on, in `fee_mint` atoms; zero when no fee
  /// applies.
  pub fee_base: u64,
  pub fee_mint: Pubkey,
}

macro_rules! route_table {
  ($(($in:ident, $out:ident, |$user:ident| $accounts:expr)),+ $(,)?) => {
    /// Every `(token_a, token_b)` pair the router serves from
    /// [`ProtocolState`], in table order.
    pub const ROUTES: &[(Pubkey, Pubkey)] = &[$(($in::MINT, $out::MINT)),+];

    /// Accounts of the exchange or earn pool instruction the router forwards
    /// `(token_a, token_b)` to, or `None` when the pair is not routable.
    #[must_use]
    pub fn route_accounts(
      user: Pubkey,
      token_a: Pubkey,
      token_b: Pubkey,
    ) -> Option<Vec<AccountMeta>> {
      $(
        if token_a == $in::MINT && token_b == $out::MINT {
          let $user = user;
          return Some($accounts.to_account_metas(None));
        }
      )+
      None
    }

    impl<C: SolanaClock> ProtocolState<C> {
      /// Quote `amount` atoms of `token_a` into `token_b` through the router.
      ///
      /// # Errors
      /// * Pair is not in [`ROUTES`]
      /// * Route gated in the current state or underlying arithmetic
      pub fn quote_route(
        &self,
        token_a: Pubkey,
        token_b: Pubkey,
        amount: u64,
      ) -> Result<RouteQuote> {
        $(
          if token_a == $in::MINT && token_b == $out::MINT {
            let out = self.output::<$in, $out>(UFix64::new(amount))?;
            return Ok(RouteQuote {
              in_amount: out.in_amount.bits,
              out_amount: out.out_amount.bits,
              fee_amount: out.fee_amount.bits,
              fee_base: out.fee_base.bits,
              fee_mint: out.fee_mint,
            });
          }
        )+
        Err(anyhow!("No Hylo route for {token_a} -> {token_b}"))
      }
    }
  };
}

route_table! {
  // LST collateral <-> hyUSD / xSOL / USDC
  (JITOSOL, HYUSD, |user| exchange::mint_stablecoin_lst(user, JITOSOL::MINT)),
  (HYLOSOL, HYUSD, |user| exchange::mint_stablecoin_lst(user, HYLOSOL::MINT)),
  (HYUSD, JITOSOL, |user| exchange::redeem_stablecoin_lst(user, JITOSOL::MINT)),
  (HYUSD, HYLOSOL, |user| exchange::redeem_stablecoin_lst(user, HYLOSOL::MINT)),
  (JITOSOL, XSOL, |user| exchange::mint_levercoin_lst(user, JITOSOL::MINT)),
  (HYLOSOL, XSOL, |user| exchange::mint_levercoin_lst(user, HYLOSOL::MINT)),
  (XSOL, JITOSOL, |user| exchange::redeem_levercoin_lst(user, JITOSOL::MINT)),
  (XSOL, HYLOSOL, |user| exchange::redeem_levercoin_lst(user, HYLOSOL::MINT)),
  (JITOSOL, USDC, |user| exchange::swap_lst_to_usdc(user, JITOSOL::MINT, JITOSOL::POOL_STATE)),
  (HYLOSOL, USDC, |user| exchange::swap_lst_to_usdc(user, HYLOSOL::MINT, HYLOSOL::POOL_STATE)),
  (USDC, JITOSOL, |user| exchange::swap_usdc_to_lst(user, JITOSOL::MINT, JITOSOL::POOL_STATE)),
  (USDC, HYLOSOL, |user| exchange::swap_usdc_to_lst(user, HYLOSOL::MINT, HYLOSOL::POOL_STATE)),
  // LST <-> LST
  (JITOSOL, HYLOSOL, |user| exchange::swap_lst_to_lst(user, JITOSOL::MINT, HYLOSOL::MINT)),
  (HYLOSOL, JITOSOL, |user| exchange::swap_lst_to_lst(user, HYLOSOL::MINT, JITOSOL::MINT)),
  // hyUSD <-> xSOL
  (HYUSD, XSOL, |user| exchange::convert_stable_to_lever_lst(user)),
  (XSOL, HYUSD, |user| exchange::convert_lever_to_stable_lst(user)),
  // USDC <-> hyUSD
  (USDC, HYUSD, |user| exchange::mint_stablecoin_usdc(user)),
  (HYUSD, USDC, |user| exchange::redeem_stablecoin_usdc(user)),
  // Earn pool
  (HYUSD, SHYUSD, |user| earn_pool::deposit(user)),
  (SHYUSD, HYUSD, |user| earn_pool::withdraw(user)),
  // cbBTC collateral <-> hyUSD / xBTC / USDC
  (CBBTC, HYUSD, |user| exchange::mint_stablecoin_exo(user, CBBTC::MINT, CBBTC::FEED.address)),
  (HYUSD, CBBTC, |user| exchange::redeem_stablecoin_exo(user, CBBTC::MINT, CBBTC::FEED.address)),
  (CBBTC, XBTC, |user| exchange::mint_levercoin_exo(user, CBBTC::MINT, CBBTC::FEED.address)),
  (XBTC, CBBTC, |user| exchange::redeem_levercoin_exo(user, CBBTC::MINT, CBBTC::FEED.address)),
  (HYUSD, XBTC, |user| exchange::convert_stable_to_lever_exo(user, CBBTC::MINT, CBBTC::FEED.address)),
  (XBTC, HYUSD, |user| exchange::convert_lever_to_stable_exo(user, CBBTC::MINT, CBBTC::FEED.address)),
  (CBBTC, USDC, |user| exchange::swap_exo_to_usdc(user, CBBTC::MINT, CBBTC::FEED.address)),
  (USDC, CBBTC, |user| exchange::swap_usdc_to_exo(user, CBBTC::MINT, CBBTC::FEED.address)),
  // HYPE collateral <-> hyUSD / xHYPE / USDC
  (HYPE, HYUSD, |user| exchange::mint_stablecoin_exo(user, HYPE::MINT, HYPE::FEED.address)),
  (HYUSD, HYPE, |user| exchange::redeem_stablecoin_exo(user, HYPE::MINT, HYPE::FEED.address)),
  (HYPE, XHYPE, |user| exchange::mint_levercoin_exo(user, HYPE::MINT, HYPE::FEED.address)),
  (XHYPE, HYPE, |user| exchange::redeem_levercoin_exo(user, HYPE::MINT, HYPE::FEED.address)),
  (HYUSD, XHYPE, |user| exchange::convert_stable_to_lever_exo(user, HYPE::MINT, HYPE::FEED.address)),
  (XHYPE, HYUSD, |user| exchange::convert_lever_to_stable_exo(user, HYPE::MINT, HYPE::FEED.address)),
  (HYPE, USDC, |user| exchange::swap_exo_to_usdc(user, HYPE::MINT, HYPE::FEED.address)),
  (USDC, HYPE, |user| exchange::swap_usdc_to_exo(user, HYPE::MINT, HYPE::FEED.address)),
}

/// Every mint that appears in [`ROUTES`].
pub const ROUTE_MINTS: &[Pubkey] = &[
  JITOSOL::MINT,
  HYLOSOL::MINT,
  HYUSD::MINT,
  SHYUSD::MINT,
  XSOL::MINT,
  USDC::MINT,
  CBBTC::MINT,
  XBTC::MINT,
  HYPE::MINT,
  XHYPE::MINT,
];

#[cfg(test)]
mod tests {
  use std::collections::BTreeSet;

  use super::*;

  #[test]
  fn every_route_has_accounts_and_distinct_pair() {
    let user = Pubkey::new_unique();
    let mut seen = BTreeSet::new();
    for &(a, b) in ROUTES {
      assert!(seen.insert((a, b)), "duplicate route {a} -> {b}");
      let accounts = route_accounts(user, a, b).expect("route has accounts");
      assert!(
        accounts.iter().any(|m| m.pubkey == user),
        "{a} -> {b} omits user"
      );
    }
    assert_eq!(ROUTES.len(), 36);
  }

  #[test]
  fn route_mints_match_table() {
    let table: BTreeSet<Pubkey> =
      ROUTES.iter().flat_map(|&(a, b)| [a, b]).collect();
    let listed: BTreeSet<Pubkey> = ROUTE_MINTS.iter().copied().collect();
    assert_eq!(table, listed);
    assert_eq!(
      listed.len(),
      ROUTE_MINTS.len(),
      "ROUTE_MINTS has a duplicate"
    );
  }

  #[test]
  fn unknown_pair_is_none() {
    let user = Pubkey::new_unique();
    assert!(route_accounts(user, XBTC::MINT, XHYPE::MINT).is_none());
    assert!(route_accounts(user, HYUSD::MINT, HYUSD::MINT).is_none());
  }
}
