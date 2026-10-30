# The connector, built natively for the architecture being built: the Supervisor builds on the
# machine that runs the add-on, and the Rust image is multi-arch. Tag and index digest pinned.
FROM rust:1.98.1-alpine3.24@sha256:7cc1c22d77d9432f7fe012a70e6d3e555af54c2a6832700ed7d553f1769ae89f AS connector

SHELL ["/bin/ash", "-o", "pipefail", "-c"]

ARG BUILD_ARCH=amd64
WORKDIR /src
COPY connector/Cargo.toml connector/Cargo.lock ./
COPY connector/src ./src
COPY connector/web ./web
COPY connector/fixtures ./fixtures
RUN case "${BUILD_ARCH}" in \
      aarch64) TARGET=aarch64-unknown-linux-musl ;; \
      amd64)   TARGET=x86_64-unknown-linux-musl ;; \
      *)       echo "unsupported BUILD_ARCH ${BUILD_ARCH}" >&2; exit 1 ;; \
    esac \
 && cargo build --release --locked --target "${TARGET}" \
 && install -m 755 "target/${TARGET}/release/connector" /connector

# Home Assistant's base image is published for amd64 and arm64 under one tag; the build's
# platform picks the matching one.
FROM ghcr.io/home-assistant/base:3.24

# The checksum check below pipes into sha256sum; failures must propagate.
SHELL ["/bin/ash", "-o", "pipefail", "-c"]

ARG BUILD_ARCH=amd64
ARG CLOUDFLARED_VERSION=2026.10.4
ARG CLOUDFLARED_SHA256_AMD64=ae5e8add7884b0531d66bbca4570d1de491ff6da51e4ebe7eb160cdf241d9e42
ARG CLOUDFLARED_SHA256_AARCH64=2b7203b4fb0ba28a0c1e12f8855ce2c25f76d266a8d5edafac5540b6812acff6

# Official static binary, pinned and checksum-verified so a build cannot pick
# up a different one than was tested.
RUN apk add --no-cache ca-certificates curl \
 && case "${BUILD_ARCH}" in \
      aarch64) CF_ARCH=arm64; CF_SHA="${CLOUDFLARED_SHA256_AARCH64}" ;; \
      amd64)   CF_ARCH=amd64; CF_SHA="${CLOUDFLARED_SHA256_AMD64}" ;; \
      *)       echo "unsupported BUILD_ARCH ${BUILD_ARCH}" >&2; exit 1 ;; \
    esac \
 && curl -fsSL -o /usr/local/bin/cloudflared \
      "https://github.com/cloudflare/cloudflared/releases/download/${CLOUDFLARED_VERSION}/cloudflared-linux-${CF_ARCH}" \
 && echo "${CF_SHA}  /usr/local/bin/cloudflared" | sha256sum -c - \
 && chmod +x /usr/local/bin/cloudflared \
 && /usr/local/bin/cloudflared --version

COPY --from=connector /connector /usr/local/bin/connector
RUN /usr/local/bin/connector --version

CMD ["/usr/local/bin/connector"]

ARG BUILD_VERSION
LABEL \
    io.hass.name="Cloudflared Connector" \
    io.hass.description="Connector for a Cloudflare Tunnel that is managed in the Zero Trust dashboard" \
    io.hass.arch="${BUILD_ARCH}" \
    io.hass.type="addon" \
    io.hass.version="${BUILD_VERSION}" \
    maintainer="Lewis Heidrick" \
    org.opencontainers.image.title="Cloudflared Connector" \
    org.opencontainers.image.description="Connector for a Cloudflare Tunnel that is managed in the Zero Trust dashboard" \
    org.opencontainers.image.vendor="Lewis Heidrick" \
    org.opencontainers.image.authors="Lewis Heidrick" \
    org.opencontainers.image.licenses="MIT" \
    org.opencontainers.image.url="https://github.com/heidrickla/ha-cloudflared-connector" \
    org.opencontainers.image.source="https://github.com/heidrickla/ha-cloudflared-connector" \
    org.opencontainers.image.documentation="https://github.com/heidrickla/ha-cloudflared-connector/blob/main/cloudflared_connector/DOCS.md" \
    org.opencontainers.image.version="${BUILD_VERSION}"
