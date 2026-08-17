fn main() {
    #[cfg(target_os = "windows")]
    {
        use std::path::PathBuf;

        // Cargo.toml is the single source of truth for the version. The resource
        // script and the manifest both receive it from here, so nothing that
        // ships can carry a version literal that drifts from the crate.
        let version =
            std::env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION is set by cargo");
        let mut parts: Vec<&str> = version.split('.').collect();
        // VERSIONINFO and the manifest both want four fields; semver supplies three.
        while parts.len() < 4 {
            parts.push("0");
        }
        let comma_version = parts[..4].join(",");
        let dotted_version = parts[..4].join(".");

        // The manifest is generated into OUT_DIR rather than the repo: a
        // generated copy under version control is just a new place for the
        // version to go stale.
        let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR is set by cargo"));
        let template = std::fs::read_to_string("resources/app.manifest.in")
            .expect("resources/app.manifest.in is readable");
        assert!(
            template.contains("{{FE_VERSION_4}}"),
            "resources/app.manifest.in lost its {{FE_VERSION_4}} placeholder — the manifest would \
             ship without a version"
        );
        let manifest = template.replace("{{FE_VERSION_4}}", &dotted_version);
        let manifest_path = out_dir.join("app.manifest");
        std::fs::write(&manifest_path, manifest).expect("generated manifest is writable");

        // Forward slashes: rc.exe treats a backslash inside a string as an
        // escape, so a Windows path would be mangled on the way in.
        let manifest_rc_path = manifest_path.to_string_lossy().replace('\\', "/");

        embed_resource::compile(
            "resources/app.rc",
            [
                format!("FE_VERSION_COMMA={}", comma_version),
                format!("FE_VERSION_STR=\"{}\"", version),
                format!("FE_MANIFEST_PATH=\"{}\"", manifest_rc_path),
            ],
        );

        println!("cargo:rerun-if-changed=resources/app.rc");
        println!("cargo:rerun-if-changed=resources/app.manifest.in");
        // Declaring any rerun-if-changed opts out of cargo's default "rerun on
        // any package change", so the version is watched explicitly. Note this
        // reruns the script; cargo can still consider the crate itself fresh
        // when moving *back* to a version it has cached artifacts for, leaving
        // the previous resource embedded. Release builds assert the embedded
        // version against Cargo.toml for exactly that reason.
        println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION");
    }
}
