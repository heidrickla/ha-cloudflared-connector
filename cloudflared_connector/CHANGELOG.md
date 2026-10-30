# Changelog

## 1.2.1 - 2026-10-30

- The tunnel token option also takes the whole install command Zero Trust shows (`cloudflared service install <token>` or `... run --token <token>`); the add-on uses the token in it.
- With no token set, the panel and the log say where in Zero Trust to find it.
- The image builds from Home Assistant's multi-architecture base image named in the Dockerfile; `build.yaml` is gone. The base image is the same 3.24 image as before.
- `config.yaml` leaves the Supervisor's defaults implicit and drops the deprecated `advanced` key.

## 1.2.0 - 2026-10-29

- A management panel in the Home Assistant sidebar (ingress): connection state, each edge connection with its location, protocol and round trip, request and response counts, the public hostnames the dashboard configured, the cloudflared log, and a restart button.
- The add-on's process is `connector`, a Rust binary that runs cloudflared as its child, restarts it with backoff when it exits, and stops it with the add-on. The tunnel token still reaches cloudflared only through `TUNNEL_TOKEN`. cloudflared's output reaches the add-on log unchanged.
- The image is built in two stages; the connector is compiled at install for the machine's architecture. `run.sh` is gone.

## 1.1.6 - 2026-10-26

- cloudflared 2026.10.4, SHA-256 verified per architecture at build. Updating the add-on rebuilds it with the new binary.

## 1.1.5 - 2026-09-29

- Base image `ghcr.io/home-assistant/{arch}-base:3.24` (Alpine 3.24), from 3.21.

## 1.1.4 - 2026-09-29

- cloudflared 2026.9.3, SHA-256 verified per architecture at build.

## 1.1.3 - 2026-09-15

- Documentation and comments cut to facts.

## 1.1.2 - 2026-09-15

- Set `pipefail` for the checksum verification pipeline (hadolint DL4006).
- Document why `BUILD_FROM` cannot be tagged inline (hadolint DL3006).

## 1.1.1 - 2026-09-15

- Add an AppArmor profile. Security rating 8.
- cloudflared runs in the add-on's own profile, not a child profile: the child transition segfaults the static Go binary on Home Assistant OS.

## 1.1.0 - 2026-09-15

- Drop host networking. Origins are reached over Home Assistant's internal network by hostname or LAN address.
- Pin cloudflared to 2026.9.1, SHA-256 verified per architecture at build.
- Pin the base images in `build.yaml`.
- Add `DOCS.md`, option translations and OCI image labels.

## 1.0.2 - 2026-09-15

- Maintainer's artwork for `icon.png` and `logo.png`.

## 1.0.1 - 2026-09-15

- Add `icon.png` and `logo.png`.

## 1.0.0 - 2026-09-15

- First release. Runs the connector for a dashboard-managed tunnel from the `tunnel_token` option, passed via `TUNNEL_TOKEN`.
