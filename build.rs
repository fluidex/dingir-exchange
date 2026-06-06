fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .compile_protos(
            &["proto/exchange/matchengine.proto", "proto/rollup/rollup.proto"],
            &["proto", "proto/third_party/googleapis"],
        )?;
    Ok(())
}
