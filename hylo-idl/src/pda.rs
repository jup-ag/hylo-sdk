use const_crypto::ed25519;
use solana_pubkey::Pubkey;
use solana_sdk_ids::address_lookup_table;

use crate::tokens::{HYUSD, SHYUSD, XSOL};
use crate::{hylo_exchange, hylo_stability_pool, MPL_TOKEN_METADATA_ID};

macro_rules! pda {
  ($program_id:expr, $base:expr) => {
    Pubkey::find_program_address(&[$base.as_ref()], &$program_id).0
  };
  ($program_id:expr, $base:expr, $key:expr) => {
    Pubkey::find_program_address(&[$base.as_ref(), $key.as_ref()], &$program_id)
      .0
  };
}

#[macro_export]
macro_rules! ata {
  ($auth:expr, $mint:expr) => {
    spl_associated_token_account_interface::address::get_associated_token_address(&$auth, &$mint)
  };
}

#[must_use]
pub fn metadata(mint: Pubkey) -> Pubkey {
  Pubkey::find_program_address(
    &[
      "metadata".as_ref(),
      MPL_TOKEN_METADATA_ID.as_ref(),
      mint.as_ref(),
    ],
    &MPL_TOKEN_METADATA_ID,
  )
  .0
}

#[must_use]
pub fn hyusd_ata(auth: Pubkey) -> Pubkey {
  ata!(&auth, &HYUSD)
}

#[must_use]
pub fn xsol_ata(auth: Pubkey) -> Pubkey {
  ata!(&auth, &XSOL)
}

#[must_use]
pub fn shyusd_ata(auth: Pubkey) -> Pubkey {
  ata!(&auth, &SHYUSD)
}

#[must_use]
pub fn vault(mint: Pubkey) -> Pubkey {
  ata!(&vault_auth(mint), &mint)
}

#[must_use]
pub fn vault_auth(mint: Pubkey) -> Pubkey {
  pda!(
    hylo_exchange::ID,
    hylo_exchange::constants::VAULT_AUTH,
    mint
  )
}

#[must_use]
pub fn new_lst_registry(slot: u64) -> Pubkey {
  Pubkey::find_program_address(
    &[LST_REGISTRY_AUTH.as_ref(), &slot.to_le_bytes()],
    &address_lookup_table::ID,
  )
  .0
}

#[must_use]
pub fn lst_header(mint: Pubkey) -> Pubkey {
  pda!(
    hylo_exchange::ID,
    hylo_exchange::constants::LST_HEADER,
    mint
  )
}

#[must_use]
pub fn fee_vault(mint: Pubkey) -> Pubkey {
  ata!(&fee_auth(mint), &mint)
}

#[must_use]
pub fn fee_auth(mint: Pubkey) -> Pubkey {
  pda!(hylo_exchange::ID, hylo_exchange::constants::FEE_AUTH, mint)
}

pub const HYLO: Pubkey = Pubkey::new_from_array(
  ed25519::derive_program_address(
    &[&hylo_exchange::constants::HYLO],
    hylo_exchange::ID.as_array(),
  )
  .0,
);

pub const HYUSD_AUTH: Pubkey = Pubkey::new_from_array(
  ed25519::derive_program_address(
    &[&hylo_exchange::constants::MINT_AUTH, HYUSD.as_array()],
    hylo_exchange::ID.as_array(),
  )
  .0,
);

pub const XSOL_AUTH: Pubkey = Pubkey::new_from_array(
  ed25519::derive_program_address(
    &[&hylo_exchange::constants::MINT_AUTH, XSOL.as_array()],
    hylo_exchange::ID.as_array(),
  )
  .0,
);

pub const LST_REGISTRY_AUTH: Pubkey = Pubkey::new_from_array(
  ed25519::derive_program_address(
    &[&hylo_exchange::constants::LST_REGISTRY_AUTH],
    hylo_exchange::ID.as_array(),
  )
  .0,
);

pub const EXCHANGE_EVENT_AUTH: Pubkey = Pubkey::new_from_array(
  ed25519::derive_program_address(
    &[b"__event_authority"],
    hylo_exchange::ID.as_array(),
  )
  .0,
);

pub const STABILITY_POOL_EVENT_AUTH: Pubkey = Pubkey::new_from_array(
  ed25519::derive_program_address(
    &[b"__event_authority"],
    hylo_stability_pool::ID.as_array(),
  )
  .0,
);

pub const POOL_CONFIG: Pubkey = Pubkey::new_from_array(
  ed25519::derive_program_address(
    &[&hylo_stability_pool::constants::POOL_CONFIG],
    hylo_stability_pool::ID.as_array(),
  )
  .0,
);

pub const SHYUSD_AUTH: Pubkey = Pubkey::new_from_array(
  ed25519::derive_program_address(
    &[&hylo_exchange::constants::MINT_AUTH, SHYUSD.as_array()],
    hylo_stability_pool::ID.as_array(),
  )
  .0,
);

pub const POOL_AUTH: Pubkey = Pubkey::new_from_array(
  ed25519::derive_program_address(
    &[&hylo_stability_pool::constants::POOL_AUTH],
    hylo_stability_pool::ID.as_array(),
  )
  .0,
);

pub const HYUSD_POOL: Pubkey = Pubkey::new_from_array(
  ed25519::derive_program_address(
    &[
      POOL_AUTH.as_array(),
      spl_token_interface::ID.as_array(),
      HYUSD.as_array(),
    ],
    spl_associated_token_account_interface::program::ID.as_array(),
  )
  .0,
);

pub const XSOL_POOL: Pubkey = Pubkey::new_from_array(
  ed25519::derive_program_address(
    &[
      POOL_AUTH.as_array(),
      spl_token_interface::ID.as_array(),
      XSOL.as_array(),
    ],
    spl_associated_token_account_interface::program::ID.as_array(),
  )
  .0,
);

pub const STABILITY_POOL_PROGRAM_DATA: Pubkey = Pubkey::new_from_array(
  ed25519::derive_program_address(
    &[hylo_stability_pool::ID.as_array()],
    solana_sdk_ids::bpf_loader_upgradeable::ID.as_array(),
  )
  .0,
);

pub const EXCHANGE_PROGRAM_DATA: Pubkey = Pubkey::new_from_array(
  ed25519::derive_program_address(
    &[hylo_exchange::ID.as_array()],
    solana_sdk_ids::bpf_loader_upgradeable::ID.as_array(),
  )
  .0,
);

pub const SOL_USD_PYTH_FEED: Pubkey =
  Pubkey::from_str_const("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE");
