FROM rust:1.88-bookworm AS builder
ARG VCS_REF=unknown
ARG TARGETARCH
WORKDIR /source
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN apt-get update \
    && apt-get install -y --no-install-recommends musl-tools \
    && rm -rf /var/lib/apt/lists/*
RUN set -eux; \
    case "$TARGETARCH" in \
      amd64) rust_target="x86_64-unknown-linux-musl" ;; \
      arm64) rust_target="aarch64-unknown-linux-musl" ;; \
      *) echo "unsupported TARGETARCH: $TARGETARCH" >&2; exit 1 ;; \
    esac; \
    rustup target add "$rust_target"; \
    CC_x86_64_unknown_linux_musl=musl-gcc \
    CC_aarch64_unknown_linux_musl=musl-gcc \
    CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=musl-gcc \
    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=musl-gcc \
    LLMAP_BUILD_SHA="$VCS_REF" \
      cargo build --release --locked --target "$rust_target"; \
    install -d -o 10001 -g 10001 /runtime/var/lib/llmap /runtime/etc/llmap; \
    install -m 0755 "target/$rust_target/release/llmap" /runtime/llmap

FROM gcr.io/distroless/static-debian12:nonroot@sha256:afa5c872c891853ca7fcf1f12c3edb23f7eeef36189728842dd51042ff57f7ab
ARG VCS_REF=unknown
ARG VERSION=development
LABEL org.opencontainers.image.source="https://github.com/IggyGG/llm-multiaccount-proxy" \
      org.opencontainers.image.revision="$VCS_REF" \
      org.opencontainers.image.version="$VERSION" \
      org.opencontainers.image.licenses="Apache-2.0"
COPY --from=builder --chown=10001:10001 /runtime/var/lib/llmap /var/lib/llmap
COPY --from=builder --chown=10001:10001 /runtime/etc/llmap /etc/llmap
COPY --from=builder /runtime/llmap /usr/local/bin/llmap
USER 10001:10001
WORKDIR /var/lib/llmap
EXPOSE 8080 8081
ENTRYPOINT ["/usr/local/bin/llmap"]
CMD ["serve", "--config", "/etc/llmap/llmap.toml"]
