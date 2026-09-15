# Changelog

## 1.1.3 - 2026-09-15

- Documentation and comments cut to facts.

## 1.1.2 - 2026-09-15

- Set `pipefail` for the checksum verification pipeline (hadolint DL4006).
- Document why `BUILD_FROM` cannot be tagged inline (hadolint DL3006).

## 1.1.1 - 2026-09-15

- Add an AppArmor profile. Security rating 8.
- cloudflared runs in the add-on's own profile, not a child profile: the child
  transition segfaults the static Go binary on Home Assistant OS.

## 1.1.0 - 2026-09-15

- Drop host networking. Origins are reached over Home Assistant's internal
  network by hostname or LAN address.
- Pin cloudflared to 2026.9.1, SHA-256 verified per architecture at build.
- Pin the base images in `build.yaml`.
- Add `DOCS.md`, option translations and OCI image labels.

## 1.0.2 - 2026-09-15

- Maintainer's artwork for `icon.png` and `logo.png`.

## 1.0.1 - 2026-09-15

- Add `icon.png` and `logo.png`.

## 1.0.0 - 2026-09-15

- First release. Runs the connector for a dashboard-managed tunnel from the
  `tunnel_token` option, passed via `TUNNEL_TOKEN`.
