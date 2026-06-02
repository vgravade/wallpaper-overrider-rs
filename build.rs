fn main() {
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=app.manifest");

    let manifest = std::fs::read_to_string("app.manifest").expect("failed to read app.manifest");
    let cargo_version = std::env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION is not set");
    let expected_manifest_version = format!(r#"version="{cargo_version}.0""#);
    assert!(
        manifest.contains(&expected_manifest_version),
        "app.manifest assemblyIdentity version must match Cargo.toml version ({cargo_version}.0)"
    );

    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        println!("cargo:rustc-link-arg-bins=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg-bins=/MANIFESTINPUT:app.manifest");
    }
}
