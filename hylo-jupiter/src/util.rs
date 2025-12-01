use anchor_lang::prelude::{AccountDeserialize, Pubkey};
use anyhow::{anyhow, Result};
use fix::prelude::*;
use fix::typenum::{IsLess, NInt, NonZero, Unsigned, U20};
use jupiter_amm_interface::AccountMap;
use rust_decimal::Decimal;
use solana_program_pack::{IsInitialized, Pack};

/// Computes fee percentage in Jupiter's favored `Decimal` type.
///
/// # Errors
/// * Arithmetic error for percentage
/// * u64 to i64 conversion
pub fn fee_pct_decimal<Exp>(
  fees_extracted: UFix64<NInt<Exp>>,
  total_in: UFix64<NInt<Exp>>,
) -> Result<Decimal>
where
  Exp: Unsigned + NonZero + IsLess<U20>,
{
  let pct_fix = fees_extracted
    .mul_div_floor(UFix64::one(), total_in)
    .ok_or(anyhow!("Arithmetic error in fee_pct calculation"))?;
  Ok(Decimal::new(pct_fix.bits.try_into()?, Exp::to_u32()))
}

/// Finds and deserializes an account in Jupiter's `AccountMap`.
///
/// # Errors
/// * Account not found in map
/// * Deserialization to `A` fails
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

pub fn account_spl_get<A: Pack + IsInitialized>(
  account_map: &AccountMap,
  key: &Pubkey,
) -> Result<A> {
  let account = account_map
    .get(key)
    .ok_or(anyhow!("Account not found {key}"))?;
  let out = A::unpack(&account.data.as_slice())?;
  Ok(out)
}
