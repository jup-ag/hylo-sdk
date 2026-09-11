//! Jupiter quote helpers built on protocol-state math.

use anchor_lang::prelude::{AccountDeserialize, Pubkey};
use anyhow::{anyhow, Context, Result};
use fix::num_traits::FromPrimitive;
use fix::prelude::UFix64;
use fix::typenum::Integer;
use hylo_idl::tokens::TokenMint;
use jupiter_amm_interface::{AccountMap, ClockRef, Quote, SwapMode, SwapParams};
use rust_decimal::Decimal;

use crate::protocol_state::ProtocolState;
use crate::token_operation::{OperationOutput, TokenOperation, TokenOperationExt};

/// Computes fee percentage as `Decimal`.
///
/// # Errors
/// * Conversions / arithmetic
pub fn fee_pct_decimal<Exp>(
  fees_extracted: UFix64<Exp>,
  fee_base: UFix64<Exp>,
) -> Result<Decimal> {
  if fee_base == UFix64::new(0) {
    Ok(Decimal::ZERO)
  } else {
    Decimal::from_u64(fees_extracted.bits)
      .zip(Decimal::from_u64(fee_base.bits))
      .and_then(|(num, denom)| num.checked_div(denom))
      .context("Arithmetic error in `fee_pct_decimal`")
  }
}

/// Converts [`OperationOutput`] to a Jupiter [`Quote`].
///
/// # Errors
/// * Fee decimal conversion
pub fn operation_to_quote<InExp, OutExp, FeeExp>(
  op: OperationOutput<InExp, OutExp, FeeExp>,
) -> Result<Quote>
where
  InExp: Integer,
  OutExp: Integer,
  FeeExp: Integer,
{
  let fee_pct = fee_pct_decimal(op.fee_amount, op.fee_base)?;
  Ok(Quote {
    in_amount: op.in_amount.bits,
    out_amount: op.out_amount.bits,
    fee_amount: op.fee_amount.bits,
    fee_mint: op.fee_mint,
    fee_pct,
  })
}

/// Generic Jupiter quote for any `IN -> OUT` pair.
///
/// # Errors
/// * Quote math / fee decimal conversion
pub fn quote<IN, OUT>(
  state: &ProtocolState<ClockRef>,
  amount: u64,
) -> Result<Quote>
where
  IN: TokenMint,
  OUT: TokenMint,
  ProtocolState<ClockRef>: TokenOperation<IN, OUT>,
  <ProtocolState<ClockRef> as TokenOperation<IN, OUT>>::FeeExp: Integer,
{
  let op = state.output::<IN, OUT>(UFix64::new(amount))?;
  operation_to_quote(op)
}

/// Finds and deserializes an account in Jupiter's `AccountMap`.
///
/// # Errors
/// * Account not found / deserialization fails
pub fn account_map_get<A: AccountDeserialize>(
  account_map: &AccountMap,
  key: &Pubkey,
) -> Result<A> {
  let account = account_map
    .get(key)
    .ok_or(anyhow!("Account not found {key}"))?;
  let mut bytes = account.data.as_slice();
  let out = A::try_deserialize(&mut bytes)?;
  Ok(out)
}

/// Validates Jupiter swap parameters for Hylo compatibility.
///
/// # Errors
/// * `ExactOut` mode / dynamic accounts
pub fn validate_swap_params<'a>(
  params: &'a SwapParams<'a, 'a>,
) -> Result<&'a SwapParams<'a, 'a>> {
  if params.swap_mode == SwapMode::ExactOut {
    Err(anyhow!("ExactOut not supported"))
  } else if params.missing_dynamic_accounts_as_default {
    Err(anyhow!("Dynamic accounts replacement not supported"))
  } else {
    Ok(params)
  }
}

use std::sync::Arc;

use anchor_spl::token::{Mint, TokenAccount};
use fix::prelude::{N8, N9};
use hylo_core::exchange_context::ExoExchangeContext;
use hylo_core::idl::earn_pool::accounts::PoolConfig;
use hylo_core::idl::exchange::accounts::{ExoPair, Hylo, LstHeader, UsdcPair};
use hylo_core::idl::pda;
use hylo_core::idl::tokens::{
  StakePool, CBBTC, HYLOSOL, HYPE, HYUSD, JITOSOL, SHYUSD, XSOL,
};
use hylo_core::lst::stake_pool::SplStakePool;
use hylo_core::pyth::{query_pyth_oracle, OracleConfig, HYPE_USD, SOL_USD};
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

use crate::protocol_state::UsdcExchangeState;

/// Accounts required to build a full [`ProtocolState`] for the v2 router path.
///
/// Shared by both the exchange and earn-pool Jupiter AMMs.
#[must_use]
pub fn accounts_to_update() -> Vec<Pubkey> {
  vec![
    pda::HYLO,
    HYUSD::MINT,
    XSOL::MINT,
    pda::lst_header(JITOSOL::MINT),
    pda::lst_header(HYLOSOL::MINT),
    JITOSOL::POOL_STATE,
    HYLOSOL::POOL_STATE,
    SOL_USD.address,
    SHYUSD::MINT,
    pda::HYUSD_POOL,
    pda::XSOL_POOL,
    pda::POOL_CONFIG,
    pda::exo_pair(CBBTC::MINT),
    pda::exo_vault(CBBTC::MINT),
    pda::exo_levercoin_mint(CBBTC::MINT),
    pda::BTC_USD_PYTH_FEED,
    pda::exo_pair(HYPE::MINT),
    pda::exo_vault(HYPE::MINT),
    pda::exo_levercoin_mint(HYPE::MINT),
    HYPE_USD.address,
    pda::USDC_PAIR,
    pda::USDC_USD_PYTH_FEED,
  ]
}

/// Builds a full [`ProtocolState`] from a Jupiter `AccountMap`.
///
/// Ported from the upstream `HyloJupiterPair::update` so the account-fetching
/// logic stays in the SDK rather than the aggregator.
///
/// # Errors
/// * A required account is missing or fails to deserialize
/// * Exchange-context / oracle load fails
pub fn build_protocol_state(
  clock: ClockRef,
  account_map: &AccountMap,
) -> Result<ProtocolState<ClockRef>> {
  // Core protocol state
  let hylo: Hylo = account_map_get(account_map, &pda::HYLO)?;
  let hyusd_mint: Mint = account_map_get(account_map, &HYUSD::MINT)?;
  let xsol_mint: Mint = account_map_get(account_map, &XSOL::MINT)?;
  let jitosol_header: LstHeader =
    account_map_get(account_map, &pda::lst_header(JITOSOL::MINT))?;
  let hylosol_header: LstHeader =
    account_map_get(account_map, &pda::lst_header(HYLOSOL::MINT))?;
  let sol_usd: PriceUpdateV2 = account_map_get(account_map, &SOL_USD.address)?;

  // Earn pool
  let shyusd_mint: Mint = account_map_get(account_map, &SHYUSD::MINT)?;
  let hyusd_pool: TokenAccount = account_map_get(account_map, &pda::HYUSD_POOL)?;
  let xsol_pool: TokenAccount = account_map_get(account_map, &pda::XSOL_POOL)?;
  let pool_config: PoolConfig =
    account_map_get(account_map, &pda::POOL_CONFIG)?;

  // cbBTC exo context
  let exo_pair: ExoPair =
    account_map_get(account_map, &pda::exo_pair(CBBTC::MINT))?;
  let cbbtc_vault: TokenAccount =
    account_map_get(account_map, &pda::exo_vault(CBBTC::MINT))?;
  let xbtc_mint: Mint =
    account_map_get(account_map, &pda::exo_levercoin_mint(CBBTC::MINT))?;
  let btc_usd: PriceUpdateV2 =
    account_map_get(account_map, &pda::BTC_USD_PYTH_FEED)?;
  let usdc_pair: UsdcPair = account_map_get(account_map, &pda::USDC_PAIR)?;
  let usdc_usd: PriceUpdateV2 =
    account_map_get(account_map, &pda::USDC_USD_PYTH_FEED)?;
  let exo_oracle_config = OracleConfig::new(
    exo_pair.oracle_interval_secs,
    exo_pair.oracle_conf_tolerance.try_into()?,
  );
  let total_collateral = UFix64::<N8>::new(cbbtc_vault.amount)
    .checked_convert()
    .context("cbBTC vault N8->N9 overflow")?;
  let cbbtc_exchange_context = Arc::new(
    ExoExchangeContext::load(
      clock.clone(),
      total_collateral,
      exo_pair.stablecoin_mint_threshold.try_into()?,
      exo_oracle_config,
      exo_pair.levercoin_fees.into(),
      &btc_usd,
      exo_pair.virtual_stablecoin.into(),
      Some(&xbtc_mint),
      exo_pair.sell_curve_config.into(),
      exo_pair.buy_curve_config.into(),
      exo_pair.levercoin_market_cap_limit.try_into()?,
    )
    .context("ExoExchangeContext::load")?,
  );

  // HYPE exo context. HYPE is already N9, so the vault balance is used as-is:
  // copying the cbBTC `checked_convert` here would convert cleanly and
  // misprice by 10x.
  let hype_exo_pair: ExoPair =
    account_map_get(account_map, &pda::exo_pair(HYPE::MINT))?;
  let hype_vault: TokenAccount =
    account_map_get(account_map, &pda::exo_vault(HYPE::MINT))?;
  let xhype_mint: Mint =
    account_map_get(account_map, &pda::exo_levercoin_mint(HYPE::MINT))?;
  let hype_usd: PriceUpdateV2 =
    account_map_get(account_map, &HYPE_USD.address)?;
  let hype_oracle_config = OracleConfig::new(
    hype_exo_pair.oracle_interval_secs,
    hype_exo_pair.oracle_conf_tolerance.try_into()?,
  );
  let hype_exchange_context = Arc::new(
    ExoExchangeContext::load(
      clock.clone(),
      UFix64::<N9>::new(hype_vault.amount),
      hype_exo_pair.stablecoin_mint_threshold.try_into()?,
      hype_oracle_config,
      hype_exo_pair.levercoin_fees.into(),
      &hype_usd,
      hype_exo_pair.virtual_stablecoin.into(),
      Some(&xhype_mint),
      hype_exo_pair.sell_curve_config.into(),
      hype_exo_pair.buy_curve_config.into(),
      hype_exo_pair.levercoin_market_cap_limit.try_into()?,
    )
    .context("ExoExchangeContext::load (HYPE)")?,
  );

  // USDC exchange state
  let usdc_oracle_config = OracleConfig::new(
    usdc_pair.oracle_interval_secs,
    usdc_pair.oracle_conf_tolerance.try_into()?,
  );
  let usdc_oracle = query_pyth_oracle(&clock, &usdc_usd, usdc_oracle_config)?;
  let usdc_exchange_state = UsdcExchangeState {
    usdc_usd_price: usdc_oracle.price_range()?,
    swap_fee: usdc_pair.swap_fee.try_into()?,
  };

  // Stake pools
  let jitosol_pool_state = account_map
    .get(&JITOSOL::POOL_STATE)
    .context("JitoSOL pool state not found")?;
  let jitosol_stake_pool = SplStakePool::from_bytes(&jitosol_pool_state.data)?;
  let hylosol_pool_state = account_map
    .get(&HYLOSOL::POOL_STATE)
    .context("hyloSOL pool state not found")?;
  let hylosol_stake_pool = SplStakePool::from_bytes(&hylosol_pool_state.data)?;

  ProtocolState::build(
    clock,
    &hylo,
    jitosol_header,
    hylosol_header,
    hyusd_mint,
    xsol_mint,
    shyusd_mint,
    pool_config,
    hyusd_pool,
    xsol_pool,
    &sol_usd,
    cbbtc_exchange_context,
    hype_exchange_context,
    usdc_exchange_state,
    jitosol_stake_pool,
    hylosol_stake_pool,
  )
}
