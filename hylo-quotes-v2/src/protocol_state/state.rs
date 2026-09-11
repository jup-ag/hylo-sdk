//! Protocol state types and deserialization
//!
//! Contains the `ProtocolState` struct and its construction from protocol
//! accounts.

use std::sync::Arc;

use anchor_lang::solana_program::clock::UnixTimestamp;
use anchor_spl::token::{Mint, TokenAccount};
use anyhow::{anyhow, Result};
use fix::prelude::*;
use hylo_core::asset_swap_config::AssetSwapConfig;
use hylo_core::conversion::UsdcStablecoinConversion;
use hylo_core::exchange_context::{ExoExchangeContext, LstExchangeContext};
use hylo_core::fees::controller::LevercoinFees;
use hylo_core::idl::earn_pool::accounts::PoolConfig;
use hylo_core::idl::exchange::accounts::{Hylo, LstHeader};
use hylo_core::lst::stake_pool::SplStakePool;
use hylo_core::lst::total_sol_cache::TotalSolCache;
use hylo_core::pyth::OracleConfig;
use hylo_core::solana_clock::SolanaClock;
use hylo_idl::tokens::{TokenMint, CBBTC, HYLOSOL, HYPE, JITOSOL};
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

use crate::{Exo, LST};

/// USDC exchange state for stablecoin mint/redeem.
#[derive(Clone)]
pub struct UsdcExchangeState {
  /// USDC/USD oracle price range
  pub usdc_usd_price: hylo_core::pyth::PriceRange<N9>,
  /// Swap fee extracted on USDC operations
  pub swap_fee: UFix64<N4>,
}

impl UsdcExchangeState {
  /// Builds the USDC stablecoin conversion from stored price range.
  #[must_use]
  pub fn conversion(&self) -> UsdcStablecoinConversion {
    UsdcStablecoinConversion {
      usdc_usd_price: self.usdc_usd_price,
    }
  }
}

/// Complete snapshot of Hylo protocol state
#[derive(Clone)]
pub struct ProtocolState<C: SolanaClock> {
  /// Exchange context with all protocol parameters
  pub exchange_context: LstExchangeContext<C>,

  /// `JitoSOL` LST header
  pub jitosol_header: LstHeader,

  /// `HyloSOL` LST header
  pub hylosol_header: LstHeader,

  /// HYUSD mint account
  pub hyusd_mint: Mint,

  /// XSOL mint account
  pub xsol_mint: Mint,

  /// SHYUSD mint account
  pub shyusd_mint: Mint,

  /// Earn pool configuration
  pub pool_config: PoolConfig,

  /// HYUSD earn pool token account
  pub hyusd_pool: TokenAccount,

  /// XSOL earn pool token account
  pub xsol_pool: TokenAccount,

  /// Timestamp of when this state was fetched
  pub fetched_at: UnixTimestamp,

  /// LST swap configuration
  pub lst_swap_config: AssetSwapConfig,

  /// cbBTC exo exchange context
  pub cbbtc_exchange_context: Arc<ExoExchangeContext<C>>,

  /// HYPE exo exchange context
  pub hype_exchange_context: Arc<ExoExchangeContext<C>>,

  /// USDC exchange state
  pub usdc_exchange_state: UsdcExchangeState,

  /// `JitoSOL` SPL stake pool
  pub jitosol_stake_pool: SplStakePool,

  /// `hyloSOL` SPL stake pool
  pub hylosol_stake_pool: SplStakePool,
}

impl<C: SolanaClock> ProtocolState<C> {
  /// Build `ProtocolState` from deserialized accounts and a clock.
  ///
  /// # Errors
  /// * Propagates errors from `ExchangeContext::load`.
  #[allow(clippy::too_many_arguments)]
  pub fn build(
    clock: C,
    hylo: &Hylo,
    jitosol_header: LstHeader,
    hylosol_header: LstHeader,
    hyusd_mint: Mint,
    xsol_mint: Mint,
    shyusd_mint: Mint,
    pool_config: PoolConfig,
    hyusd_pool: TokenAccount,
    xsol_pool: TokenAccount,
    sol_usd: &PriceUpdateV2,
    cbbtc_exchange_context: Arc<ExoExchangeContext<C>>,
    hype_exchange_context: Arc<ExoExchangeContext<C>>,
    usdc_exchange_state: UsdcExchangeState,
    jitosol_stake_pool: SplStakePool,
    hylosol_stake_pool: SplStakePool,
  ) -> Result<Self> {
    let fetched_at = clock.unix_timestamp();
    let total_sol_cache: TotalSolCache = hylo.total_sol_cache.into();
    let oracle_config = OracleConfig::new(
      hylo.oracle_interval_secs,
      hylo.oracle_conf_tolerance.try_into()?,
    );
    let xsol_fees: LevercoinFees = hylo.levercoin_fees.into();
    let lst_swap_config = AssetSwapConfig::new(hylo.lst_swap_fee.into())?;
    let exchange_context = LstExchangeContext::load(
      clock,
      &total_sol_cache,
      hylo.stablecoin_mint_threshold.try_into()?,
      oracle_config,
      xsol_fees,
      sol_usd,
      hylo.virtual_stablecoin.into(),
      Some(&xsol_mint),
      hylo.lst_sell_curve_config.into(),
      hylo.lst_buy_curve_config.into(),
    )?;
    Ok(Self {
      exchange_context,
      jitosol_header,
      hylosol_header,
      hyusd_mint,
      xsol_mint,
      shyusd_mint,
      pool_config,
      hyusd_pool,
      xsol_pool,
      fetched_at,
      lst_swap_config,
      cbbtc_exchange_context,
      hype_exchange_context,
      usdc_exchange_state,
      jitosol_stake_pool,
      hylosol_stake_pool,
    })
  }

  /// Selects an [`LstHeader`] field given a token implementing [`LST`].
  ///
  /// # Errors
  /// * LST does not have a corresponding header field in this struct
  pub fn lst_header<L: LST>(&self) -> Result<&LstHeader> {
    match L::MINT {
      JITOSOL::MINT => Ok(&self.jitosol_header),
      HYLOSOL::MINT => Ok(&self.hylosol_header),
      _ => Err(anyhow!("LstHeader not found for {}", L::MINT)),
    }
  }

  /// SPL stake pool for the given LST.
  ///
  /// # Errors
  /// * Unknown LST mint
  pub fn stake_pool<L: LST>(&self) -> Result<&SplStakePool> {
    match L::MINT {
      JITOSOL::MINT => Ok(&self.jitosol_stake_pool),
      HYLOSOL::MINT => Ok(&self.hylosol_stake_pool),
      _ => Err(anyhow!("stake_pool not found for mint {}", L::MINT)),
    }
  }

  #[must_use]
  pub fn cbbtc_exchange_context(&self) -> &ExoExchangeContext<C> {
    &self.cbbtc_exchange_context
  }

  /// Selects the exo exchange context for a given collateral.
  ///
  /// # Errors
  /// * Collateral has no exo pair registered in this snapshot
  pub fn exo_exchange_context<E: Exo>(&self) -> Result<&ExoExchangeContext<C>> {
    match E::MINT {
      CBBTC::MINT => Ok(&self.cbbtc_exchange_context),
      HYPE::MINT => Ok(&self.hype_exchange_context),
      _ => Err(anyhow!("No exo pair registered for mint {}", E::MINT)),
    }
  }

  #[must_use]
  pub fn usdc_exchange_state(&self) -> &UsdcExchangeState {
    &self.usdc_exchange_state
  }
}
