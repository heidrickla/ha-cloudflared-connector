# Changelog

## 1.1.1 - 2026-09-15

- Add a custom AppArmor profile; the add-on now carries Home Assistant's
  maximum security rating. cloudflared runs inside the add-on's profile
  rather than in a child profile: the child-profile transition made the
  static Go binary segfault at start on Home Assistant OS, and the platform
  logs no AppArmor audit, so the profile was proven by running it.

## 1.1.0 - 2026-09-15

- Drop host networking. The connector reaches origins over Home Assistant's
  internal network by hostname or LAN address instead, which improves the
  add-on's security rating.
- Pin cloudflared to release 2026.9.1 and verify the downloaded binary against
  a recorded SHA-256 for each architecture at build time.
- Pin the Home Assistant base images in `build.yaml`.
- Add documentation, option translations and OCI image labels.

## 1.0.2 - 2026-09-15

- Replace the generated glyph with the maintainer's artwork: an orange cloud
  with a tunnel cut from its base and a blue plug set into the opening, on a
  navy tile.

## 1.0.1 - 2026-09-15

- Add `icon.png` and `logo.png`.

## 1.0.0 - 2026-09-15

- First release. Runs the connector for a dashboard-managed Cloudflare Tunnel
  from the `tunnel_token` option, passing the token to cloudflared through the
  `TUNNEL_TOKEN` environment variable rather than the command line.
