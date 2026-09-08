const DOCKERFILE: &str = include_str!("../Dockerfile");
const RELEASE_WORKFLOW: &str = include_str!("../.github/workflows/release.yml");

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
fn container_binary_uses_rustc_static_musl_linkage_without_raw_static_override() {
    assert!(DOCKERFILE.contains("x86_64-unknown-linux-musl"));
    assert!(DOCKERFILE.contains("aarch64-unknown-linux-musl"));
    assert!(DOCKERFILE.contains("--target \"$rust_target\""));
    assert!(DOCKERFILE.contains("-C target-feature=+crt-static"));
    assert!(
        DOCKERFILE.contains("-C relocation-model=static"),
        "rustc must disable PIE when producing the self-contained executable"
    );
    assert!(
        !DOCKERFILE.contains("link-arg=-static"),
        "a raw -static linker argument conflicts with rustc's static-PIE startup objects"
    );
    assert!(
        DOCKERFILE.contains("readelf -l /runtime/llmap"),
        "the image build must inspect the produced executable"
    );
    assert!(
        DOCKERFILE.contains("dynamically linked executable is not allowed"),
        "the image build must fail if the executable retains an interpreter"
    );
    assert!(
        DOCKERFILE.contains("readelf -d /runtime/llmap"),
        "the image build must inspect dynamic dependencies"
    );
    assert!(
        DOCKERFILE.contains("dynamically linked library is not allowed"),
        "the image build must fail if the executable retains a needed library"
    );
}

#[test]
fn release_executes_both_container_architectures_before_signing() {
    let amd64 = RELEASE_WORKFLOW
        .find("docker run --rm --platform linux/amd64 \"${IMAGE}@${DIGEST}\" --version")
        .expect("the release must execute the published amd64 image");
    let arm64 = RELEASE_WORKFLOW
        .find("docker run --rm --platform linux/arm64 \"${IMAGE}@${DIGEST}\" --version")
        .expect("the release must execute the published arm64 image through QEMU");
    let signing = RELEASE_WORKFLOW
        .find("cosign sign --yes \"${IMAGE}@${DIGEST}\"")
        .expect("the release must sign the published image");

    assert!(amd64 < signing);
    assert!(arm64 < signing);
}
