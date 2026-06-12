use std::sync::Arc;

pub mod noop;
#[cfg(feature = "zk-rollup")]
pub mod zk;

/// Abstraction for order signature verification.
///
/// In a zk-rollup scenario this is backed by BabyJubJub/EdDSA.
/// In a plain exchange scenario a [`noop::NoopVerifier`] can be used
/// to skip verification entirely, or a custom impl (e.g. ECDSA / HMAC)
/// can be plugged in.
pub trait SignatureVerifier: Send + Sync {
    /// Verify a signature.
    ///
    /// * `pubkey`   – public key string as stored in `UserInfo.l2_pubkey`
    /// * `msg`      – raw message bytes produced by `OrderCommitter::commit_order`
    /// * `signature`– hex-encoded signature string from the RPC request
    fn verify(&self, pubkey: &str, msg: &[u8], signature: &str) -> bool;
}

/// Abstraction for building the cryptographic commitment of an order.
///
/// In a zk-rollup scenario this constructs an `OrderCommitment` and
/// hashes it with Poseidon (see [`zk::ZkOrderCommitter`]).
/// In a plain exchange scenario a [`noop::NoopCommitter`] simply
/// returns empty bytes.
pub trait OrderCommitter: Send + Sync {
    fn commit_order(
        &self,
        req: &crate::rpc::exchange::OrderPutRequest,
        market: &crate::market::Market,
        asset_manager: &crate::asset::AssetManager,
    ) -> anyhow::Result<Vec<u8>>;
}

pub type DynSignatureVerifier = Arc<dyn SignatureVerifier>;
pub type DynOrderCommitter = Arc<dyn OrderCommitter>;
