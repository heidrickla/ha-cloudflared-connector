ARG BUILD_FROM=ghcr.io/home-assistant/amd64-base:3.21
FROM ${BUILD_FROM}

ARG BUILD_ARCH=amd64
ARG CLOUDFLARED_VERSION=2026.9.1
ARG CLOUDFLARED_SHA256_AMD64=03f1f25d1cc93b9ad6c60569d44060bc4f17ed97075760ed8cfca4b12dcd68cc
ARG CLOUDFLARED_SHA256_AARCH64=3d97437c71848bd8df68041e12436b484a661d95073ea1937f01a845ce88faa3

# The official static cloudflared binary, pinned to one release and verified
# against a recorded checksum, so a build cannot silently pick up a different
# binary than the one this version was tested with.
RUN apk add --no-cache ca-certificates curl jq \
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

COPY run.sh /run.sh
RUN chmod +x /run.sh

CMD ["/run.sh"]

ARG BUILD_DATE
ARG BUILD_DESCRIPTION
ARG BUILD_NAME
ARG BUILD_REF
ARG BUILD_REPOSITORY
ARG BUILD_VERSION
LABEL \
    io.hass.name="${BUILD_NAME}" \
    io.hass.description="${BUILD_DESCRIPTION}" \
    io.hass.arch="${BUILD_ARCH}" \
    io.hass.type="addon" \
    io.hass.version="${BUILD_VERSION}" \
    maintainer="Lewis Heidrick" \
    org.opencontainers.image.title="${BUILD_NAME}" \
    org.opencontainers.image.description="${BUILD_DESCRIPTION}" \
    org.opencontainers.image.vendor="Lewis Heidrick" \
    org.opencontainers.image.authors="Lewis Heidrick" \
    org.opencontainers.image.licenses="MIT" \
    org.opencontainers.image.url="https://github.com/heidrickla/ha-cloudflared-connector" \
    org.opencontainers.image.source="https://github.com/${BUILD_REPOSITORY}" \
    org.opencontainers.image.documentation="https://github.com/${BUILD_REPOSITORY}/blob/main/cloudflared_connector/DOCS.md" \
    org.opencontainers.image.created="${BUILD_DATE}" \
    org.opencontainers.image.revision="${BUILD_REF}" \
    org.opencontainers.image.version="${BUILD_VERSION}"
