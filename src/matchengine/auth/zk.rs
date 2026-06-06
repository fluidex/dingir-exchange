use super::{OrderCommitter, SignatureVerifier};
use crate::asset::AssetManager;
use crate::market::Market;
use crate::rpc::exchange::{OrderPutRequest, OrderSide};
use crate::utils::crypto::{BigInt, DecimalExt, Fr, FrExt, PubkeyExt, SignatureExt, verify as babyjubjub_verify};
use anyhow::{Result, bail};
use rust_decimal::RoundingStrategy;
use std::str::FromStr;

/// Order commitment used as the witness for zk-proof verification.
///
/// The hash is consistent with the FluiDex circuits:
/// <https://github.com/fluidex/circuits/blob/d6e06e964b9d492f1fa5513bcc2295e7081c540d/helper.ts/state-utils.ts#L38>
pub struct OrderCommitment {
    pub token_sell: Fr,
    pub token_buy: Fr,
    pub total_sell: Fr,
    pub total_buy: Fr,
}

impl OrderCommitment {
    pub fn hash(&self) -> BigInt {
        let magic_head = Fr::from_u32(4); // TxType::PlaceOrder
        let data = Fr::hash(&[magic_head, self.token_sell, self.token_buy, self.total_sell, self.total_buy]);
        data.to_bigint()
    }
}

/// BabyJubJub / EdDSA verifier.
pub struct BabyJubJubVerifier;

impl SignatureVerifier for BabyJubJubVerifier {
    fn verify(&self, pubkey: &str, msg: &[u8], signature: &str) -> bool {
        let pubkey = match PubkeyExt::from_str(pubkey) {
            Ok(pk) => pk,
            Err(_) => {
                log::error!("invalid pubkey {:?}", pubkey);
                return false;
            }
        };
        let signature = match SignatureExt::from_str(signature) {
            Ok(sig) => sig,
            Err(_) => {
                log::error!("invalid signature {:?}", signature);
                return false;
            }
        };
        let msg = BigInt::from_signed_bytes_be(msg);
        babyjubjub_verify(pubkey, signature, msg)
    }
}

/// Zk-order committer that builds a circuit-compatible `OrderCommitment`.
pub struct ZkOrderCommitter;

impl OrderCommitter for ZkOrderCommitter {
    fn commit_order(&self, o: &OrderPutRequest, market: &Market, asset_manager: &AssetManager) -> Result<Vec<u8>> {
        let assets: Vec<&str> = o.market.split('_').collect();
        if assets.len() != 2 {
            bail!("market error");
        }

        let base_token = asset_manager
            .asset_get(assets[0])
            .ok_or_else(|| anyhow::anyhow!("market base_token error"))?;
        let quote_token = asset_manager
            .asset_get(assets[1])
            .ok_or_else(|| anyhow::anyhow!("market quote_token error"))?;

        let amount = match rust_decimal::Decimal::from_str(&o.amount) {
            Ok(d) => d.round_dp_with_strategy(market.amount_prec, RoundingStrategy::ToZero),
            _ => bail!("amount error"),
        };
        let price = match rust_decimal::Decimal::from_str(&o.price) {
            Ok(d) => d.round_dp(market.price_prec),
            _ => bail!("price error"),
        };

        let commitment = match OrderSide::try_from(o.order_side) {
            Ok(OrderSide::Ask) => OrderCommitment {
                token_buy: Fr::from_u32(quote_token.inner_id),
                token_sell: Fr::from_u32(base_token.inner_id),
                total_buy: (amount * price).to_fr(market.amount_prec + market.price_prec),
                total_sell: amount.to_fr(market.amount_prec),
            },
            Ok(OrderSide::Bid) => OrderCommitment {
                token_buy: Fr::from_u32(base_token.inner_id),
                token_sell: Fr::from_u32(quote_token.inner_id),
                total_buy: amount.to_fr(market.amount_prec),
                total_sell: (amount * price).to_fr(market.amount_prec + market.price_prec),
            },
            Err(_) => bail!("market error"),
        };

        Ok(commitment.hash().to_signed_bytes_be())
    }
}
