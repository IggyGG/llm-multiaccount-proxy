const DOCKERFILE: &str = include_str!("../Dockerfile");

#[test]
fn runtime_image_is_distroless_and_contains_no_shell_package_manager() {
    assert!(
        DOCKERFILE.contains("FROM gcr.io/distroless/cc-debian12:nonroot"),
        "the runtime stage must use the minimal distroless base"
    );
    assert!(!DOCKERFILE.contains("apt-get install"));
    assert!(!DOCKERFILE.contains("useradd"));
}
