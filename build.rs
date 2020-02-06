fn main() {
    tonic_build::configure()
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .compile(&["proto/exchange/matchengine.proto"], &["proto/exchange"])
        .unwrap();
}
