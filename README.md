# Cloudflared Connector for Home Assistant

A Home Assistant add-on that runs the connector for a Cloudflare Tunnel you
manage in the Zero Trust dashboard. It holds one thing, the tunnel's connector
token, and keeps the connection up. Public hostnames, origins and Access
policies stay in Cloudflare, so exposing another service later needs no change
to the add-on.

## Install

Add this repository to your add-on store, then install Cloudflared Connector:

    https://github.com/heidrickla/ha-cloudflared-connector

Settings, Add-ons, Add-on Store, the menu in the top right, Repositories.

Paste the connector token from Zero Trust (Networks, Tunnels) into the
Tunnel token option and start the add-on. The log shows
`Registered tunnel connection` four times when it is up.

Full setup, origin addressing and troubleshooting are in the add-on's
[documentation](cloudflared_connector/DOCS.md).

## What it does and does not do

- Runs cloudflared as a connector for a dashboard-managed tunnel.
- Passes the token through the `TUNNEL_TOKEN` environment variable, never on
  the command line.
- Pins the cloudflared release it fetches and verifies it against a recorded
  SHA-256 at build time; pins the Home Assistant base images.
- Runs without host networking and under its own AppArmor profile, which
  gives it Home Assistant's maximum security rating.
- Does not configure hostnames, origins or Access. Do that in Cloudflare, and
  put an Access policy in front of anything you would not want on the open
  internet.

## Repository layout

    repository.yaml              add-on repository marker
    cloudflared_connector/       the add-on
      config.yaml                metadata, options and schema
      build.yaml                 pinned base images and the cloudflared pin
      Dockerfile                 fetches and verifies the cloudflared binary
      run.sh                     reads the options and execs cloudflared
      apparmor.txt               the add-on's AppArmor profile
      translations/en.yaml       option names and descriptions
      DOCS.md, CHANGELOG.md      shown in the add-on's Documentation and Changelog tabs
      icon.png, logo.png         store branding
    brand/                       artwork sources and the script that derives the PNGs

## Artwork

The mark and the icon-family sheet in `brand/` are the maintainer's own
artwork. `brand/generate.py` only scales the tile and adds the wordmark.

## License

MIT. See [LICENSE](LICENSE).
