# Cloudflared Connector for Home Assistant

Home Assistant add-on repository. One add-on: the connector for a Cloudflare
Tunnel configured in the Zero Trust dashboard.

Hostnames, origins and Access policies live in Cloudflare. The add-on holds
only the connector token, so exposing another service needs no change here.

## Install

Settings > Add-ons > Add-on Store > menu > Repositories, add:

    https://github.com/heidrickla/ha-cloudflared-connector

Install `Cloudflared Connector`, set `tunnel_token`, start it. Up when the log
shows `Registered tunnel connection` four times.

Setup, origin addressing and troubleshooting:
[DOCS.md](cloudflared_connector/DOCS.md).

## Properties

| | |
|---|---|
| Architectures | amd64, aarch64 |
| Security rating | 8 (AppArmor profile, no host networking) |
| cloudflared | 2026.9.1, SHA-256 verified at build |
| Base image | `ghcr.io/home-assistant/{arch}-base:3.21` |

## Layout

    repository.yaml              repository marker
    cloudflared_connector/       the add-on
    brand/                       artwork sources; generate.py derives the PNGs

Artwork in `brand/` is the maintainer's own.

## License

MIT. See [LICENSE](LICENSE).
