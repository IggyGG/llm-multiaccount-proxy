const DOCKERFILE: &str = include_str!("../Dockerfile");

#[test]
fn runtime_image_is_static_distroless_and_contains_no_shell_package_manager() {
    let runtime = DOCKERFILE
        .split("\nFROM ")
        .last()
        .expect("Dockerfile must contain a runtime stage");

    assert!(
        runtime.starts_with("gcr.io/distroless/static-debian12:nonroot@sha256:"),
        "the runtime stage must use a digest-pinned static distroless base"
    );
    assert!(!runtime.contains("apt-get"));
    assert!(!runtime.contains("useradd"));
}

#[test]
fn container_binary_is_built_for_a_static_musl_target() {
    assert!(DOCKERFILE.contains("x86_64-unknown-linux-musl"));
    assert!(DOCKERFILE.contains("aarch64-unknown-linux-musl"));
    assert!(DOCKERFILE.contains("--target \"$rust_target\""));
    assert!(DOCKERFILE.contains("-C link-arg=-static"));
    assert!(
        DOCKERFILE.contains("readelf -l /runtime/llmap"),
        "the image build must inspect the produced executable"
    );
    assert!(
        DOCKERFILE.contains("dynamically linked executable is not allowed"),
        "the image build must fail if the executable retains an interpreter"
    );
}
