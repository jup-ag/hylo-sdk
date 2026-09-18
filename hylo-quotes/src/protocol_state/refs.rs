//! [`ProtocolAccounts`] by reference, for callers that already own the
//! accounts (an aggregator's account map) and should not copy them to build
//! a [`ProtocolState`](super::ProtocolState).

use anchor_lang::prelude::Pubkey;
use anyhow::{Context, Result};
use hylo_core::error::CoreError;
use solana_account::Account;

use super::ProtocolAccounts;

macro_rules! protocol_account_refs {
  ($(($field:ident, $index:literal, $name:literal)),+ $(,)?) => {
    /// Every field of [`ProtocolAccounts`], borrowed.
    #[derive(Debug, Clone, Copy)]
    pub struct ProtocolAccountRefs<'a> {
      $(pub $field: &'a Account,)+
    }

    impl<'a> ProtocolAccountRefs<'a> {
      /// Resolve every account in [`ProtocolAccounts::PUBKEYS`] through
      /// `lookup`.
      ///
      /// # Errors
      /// * [`CoreError::ProtocolAccountNotFound`] for the first account
      ///   `lookup` does not have
      pub fn from_lookup(
        lookup: impl Fn(&Pubkey) -> Option<&'a Account>,
      ) -> Result<Self> {
        Ok(Self {
          $($field: lookup(&ProtocolAccounts::PUBKEYS[$index])
            .ok_or(CoreError::ProtocolAccountNotFound)
            .context(concat!($name, " not found"))?,)+
        })
      }
    }

    impl ProtocolAccounts {
      /// Borrow every account.
      #[must_use]
      pub fn as_refs(&self) -> ProtocolAccountRefs<'_> {
        ProtocolAccountRefs {
          $($field: &self.$field,)+
        }
      }
    }
  };
}

// Same order and names as `ProtocolAccounts::from_fetched`.
protocol_account_refs! {
  (hylo, 0, "Hylo account"),
  (jitosol_header, 1, "JitoSOL header"),
  (hylosol_header, 2, "HyloSOL header"),
  (hyusd_mint, 3, "HYUSD mint"),
  (shyusd_mint, 4, "SHYUSD mint"),
  (xsol_mint, 5, "XSOL mint"),
  (pool_config, 6, "Pool config"),
  (hyusd_pool, 7, "HYUSD pool"),
  (sol_usd_pyth, 8, "SOL/USD Pyth feed"),
  (clock, 9, "Clock sysvar"),
  (cbbtc_exo_pair, 10, "cbBTC ExoPair"),
  (cbbtc_vault, 11, "cbBTC vault"),
  (xbtc_mint, 12, "xBTC mint"),
  (btc_usd_pyth, 13, "BTC/USD Pyth feed"),
  (usdc_pair, 14, "UsdcPair"),
  (usdc_usd_pyth, 15, "USDC/USD Pyth feed"),
  (jitosol_pool_state, 16, "JitoSOL pool state"),
  (hylosol_pool_state, 17, "hyloSOL pool state"),
  (jitosol_vault, 18, "JitoSOL vault"),
  (hylosol_vault, 19, "hyloSOL vault"),
  (usdc_vault, 20, "USDC vault"),
  (hype_exo_pair, 21, "HYPE ExoPair"),
  (hype_vault, 22, "HYPE vault"),
  (xhype_mint, 23, "xHYPE mint"),
  (hype_usd_pyth, 24, "HYPE/USD Pyth feed"),
}

#[cfg(test)]
mod tests {
  use std::collections::HashMap;

  use super::*;

  fn account(tag: u8) -> Account {
    Account {
      lamports: u64::from(tag),
      data: vec![tag],
      owner: Pubkey::default(),
      executable: false,
      rent_epoch: 0,
    }
  }

  #[test]
  fn from_lookup_follows_pubkey_order() {
    let map: HashMap<Pubkey, Account> = ProtocolAccounts::PUBKEYS
      .iter()
      .enumerate()
      .map(|(i, key)| (*key, account(u8::try_from(i).unwrap())))
      .collect();
    let refs = ProtocolAccountRefs::from_lookup(|key| map.get(key)).unwrap();
    assert_eq!(refs.hylo.data, [0]);
    assert_eq!(refs.clock.data, [9]);
    assert_eq!(refs.hype_usd_pyth.data, [24]);
  }

  #[test]
  fn from_lookup_names_the_missing_account() {
    let map: HashMap<Pubkey, Account> = ProtocolAccounts::PUBKEYS
      .iter()
      .filter(|key| **key != ProtocolAccounts::PUBKEYS[14])
      .map(|key| (*key, account(0)))
      .collect();
    let err = ProtocolAccountRefs::from_lookup(|key| map.get(key)).unwrap_err();
    assert!(err.to_string().contains("UsdcPair"), "{err}");
  }

  #[test]
  fn as_refs_round_trips_from_fetched() {
    let fetched: Vec<Option<Account>> = (0..ProtocolAccounts::PUBKEYS.len())
      .map(|i| Some(account(u8::try_from(i).unwrap())))
      .collect();
    let owned = ProtocolAccounts::from_fetched(&fetched).unwrap();
    let refs = owned.as_refs();
    assert_eq!(refs.jitosol_vault.data, [18]);
    assert!(std::ptr::eq(refs.usdc_vault, &owned.usdc_vault));
  }
}
