FROM rust:1.88-bookworm AS builder
ARG VCS_REF=unknown
WORKDIR /source
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN LLMAP_BUILD_SHA="$VCS_REF" cargo build --release --locked
RUN install -d -o 10001 -g 10001 /runtime/var/lib/llmap /runtime/etc/llmap

FROM gcr.io/distroless/cc-debian12:nonroot@sha256:9dac0a79194e45a7da0158a9c6da57b217585af0786db3845d1f0ec1a0dd182f
ARG VCS_REF=unknown
ARG VERSION=development
LABEL org.opencontainers.image.source="https://github.com/IggyGG/llm-multiaccount-proxy" \
      org.opencontainers.image.revision="$VCS_REF" \
      org.opencontainers.image.version="$VERSION" \
      org.opencontainers.image.licenses="Apache-2.0"
COPY --from=builder --chown=10001:10001 /runtime/var/lib/llmap /var/lib/llmap
COPY --from=builder --chown=10001:10001 /runtime/etc/llmap /etc/llmap
COPY --from=builder /source/target/release/llmap /usr/local/bin/llmap
USER 10001:10001
WORKDIR /var/lib/llmap
EXPOSE 8080 8081
ENTRYPOINT ["/usr/local/bin/llmap"]
CMD ["serve", "--config", "/etc/llmap/llmap.toml"]
