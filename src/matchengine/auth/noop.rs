use super::{OrderCommitter, SignatureVerifier};
use crate::asset::AssetManager;
use crate::market::Market;
use crate::rpc::exchange::OrderPutRequest;

/// A verifier that always succeeds.
///
/// Suitable for non-zk deployments where order signature checking is
/// handled externally or not required.
pub struct NoopVerifier;

impl SignatureVerifier for NoopVerifier {
    fn verify(&self, _pubkey: &str, _msg: &[u8], _signature: &str) -> bool {
        true
    }
}

/// A committer that returns empty bytes.
///
/// Suitable for non-zk deployments where no circuit-friendly order
/// commitment is needed.
pub struct NoopCommitter;

impl OrderCommitter for NoopCommitter {
    fn commit_order(
        &self,
        _req: &OrderPutRequest,
        _market: &Market,
        _asset_manager: &AssetManager,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(Vec::new())
    }
}
