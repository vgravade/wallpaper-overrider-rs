fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=app.manifest");
    println!("cargo:rerun-if-changed=app.rc");
    println!("cargo:rerun-if-changed=assets/app.ico");

    let manifest = std::fs::read_to_string("app.manifest")?;
    let cargo_version = std::env::var("CARGO_PKG_VERSION")?;
    let expected_manifest_version = format!(r#"version="{cargo_version}.0""#);
    if !manifest.contains(&expected_manifest_version) {
        return Err(format!(
            "app.manifest assemblyIdentity version must match Cargo.toml version ({cargo_version}.0)"
        )
        .into());
    }

    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        embed_resource::compile("app.rc", embed_resource::NONE)
            .manifest_required()
            .map_err(|e| format!("failed to compile app.rc: {e}"))?;
        println!("cargo:rustc-link-arg-bins=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg-bins=/MANIFESTINPUT:app.manifest");
    }

    Ok(())
}
