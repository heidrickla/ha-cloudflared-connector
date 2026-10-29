# Cloudflared Connector for Home Assistant

Home Assistant add-on repository. One add-on: the connector for a Cloudflare Tunnel configured in the Zero Trust dashboard.

Hostnames, origins and Access policies live in Cloudflare. The add-on holds only the connector token, so exposing another service needs no change here.

## Install

Settings > Add-ons > Add-on Store > menu > Repositories, add:

    https://github.com/heidrickla/ha-cloudflared-connector

Install `Cloudflared Connector`, set `tunnel_token`, start it. Up when the panel shows four edge connections, or the log shows `Registered tunnel connection` four times.

Setup, origin addressing and troubleshooting: [DOCS.md](cloudflared_connector/DOCS.md).

## Panel

The add-on adds a Cloudflared page to the Home Assistant sidebar. It refreshes every 5 seconds.

| Tab | Shows |
|---|---|
| Overview | State, edge connections with location, protocol, round trip and bytes, requests and responses by status code, cloudflared version, connector ID, configuration version, restarts, a restart button |
| Hostnames | The public hostnames, paths and services the Zero Trust dashboard configured, as cloudflared is running them |
| Log | The last 300 lines cloudflared printed, filtered by level |

![Overview](docs/screenshots/overview.png)

![Hostnames](docs/screenshots/hostnames.png)

![Log](docs/screenshots/log.png)

The screenshots show the panel's demo mode (`connector --demo`), fed with example data.

## Properties

| | |
|---|---|
| Architectures | amd64, aarch64 |
| Security rating | 8 (AppArmor profile, no host networking) |
| cloudflared | 2026.10.4, SHA-256 verified at build |
| Base image | `ghcr.io/home-assistant/{arch}-base:3.24` |
| Connector | Rust, built at install from `cloudflared_connector/connector/` in `rust:1.98.1-alpine3.24` |
| Panel | Ingress only; answers the Supervisor's ingress proxy and nothing else |
| Gates | `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo deny check`, yamllint, hadolint, both image builds |

## Layout

    repository.yaml              repository marker
    cloudflared_connector/       the add-on
    cloudflared_connector/connector/  the connector: supervisor, status model, panel
    docs/screenshots/            panel screenshots
    brand/                       artwork sources; generate.py derives the PNGs

Artwork in `brand/` is the maintainer's own.

## License

MIT. See [LICENSE](LICENSE).
