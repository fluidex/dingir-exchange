fn build_grpc() {
    tonic_build::configure()
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .field_attribute(".matchengine.AssetListResponse.AssetInfo.chain_id", "#[serde(rename = \"chainId\")]")
        .field_attribute(".matchengine.AssetListResponse.AssetInfo.token_address", "#[serde(rename = \"address\")]")
        .field_attribute(".matchengine.AssetListResponse.AssetInfo.logo_uri", "#[serde(rename = \"logoURI\")]")
        .compile(
            &["proto/exchange/matchengine.proto"],
            &["proto/exchange", "proto/third_party/googleapis"],
        )
        .unwrap();
}

fn main() {
    build_grpc()
}
