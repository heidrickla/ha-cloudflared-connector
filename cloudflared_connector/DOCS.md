# Cloudflared Connector

Runs the connector side of a Cloudflare Tunnel that you manage in the Cloudflare
Zero Trust dashboard. The add-on holds one thing, the tunnel's connector token,
and keeps the connection to Cloudflare up. Everything else about the tunnel,
its public hostnames, the origins they route to, and the Access policies that
gate them, lives in Cloudflare and is pulled by the connector automatically.

Because the configuration is remote, exposing another service later needs no
change to this add-on: add the hostname in Cloudflare and it is live.

## Before you start

1. In Cloudflare Zero Trust, go to Networks, Tunnels, and create a tunnel with
   the Cloudflare (dashboard) configuration option.
2. Copy the connector token shown on the install step. It is a long
   base64-looking string. You do not need to run the install command Cloudflare
   shows; this add-on is that command.
3. Add a public hostname to the tunnel and point it at your service (see
   Addressing origins below).
4. Put a Cloudflare Access application in front of the hostname so that only
   you can reach it. A tunnel alone makes a service public.

## Installation

1. Add this repository to the add-on store: Settings, Add-ons, Add-on Store,
   the menu in the top right, Repositories, then paste the repository URL.
2. Install Cloudflared Connector.
3. Open Configuration, paste the connector token into Tunnel token, and save.
4. Start the add-on. Within a few seconds the log shows
   `Registered tunnel connection` four times, once per edge connection, and
   the tunnel shows as Healthy in Zero Trust.

## Configuration

| Option | Required | Default | Meaning |
|---|---|---|---|
| `tunnel_token` | yes | | The connector token for your dashboard-managed tunnel |
| `loglevel` | no | `info` | cloudflared log level: `debug`, `info`, `warn` or `error` |

The token is handed to cloudflared through the `TUNNEL_TOKEN` environment
variable, not on the command line, so it does not appear in a process listing.
It is stored in the add-on's options, which Home Assistant keeps in its own
protected storage.

## Addressing origins

The add-on runs on Home Assistant's internal network, without host networking.
From there:

- Home Assistant itself is `http://homeassistant:8123`.
- Another add-on is reachable by its hostname, which is its slug with
  underscores replaced by dashes, on the port the add-on listens on inside its
  container, for example `http://a0d7b954-bookstack:80`.
- A service published on the Home Assistant host's LAN address, or anything
  else on your network, is reachable by that address and port, for example
  `http://192.168.1.10:2665`.

Set the origin for each public hostname in Zero Trust accordingly.

## Security notes

- Do not publish a hostname without an Access policy unless the service has
  its own strong authentication and you accept it being internet-facing.
- One token, one tunnel. If the token leaks, rotate it in Zero Trust
  (delete and recreate the tunnel's token) and update the add-on option.
- The add-on runs without host networking and under its own AppArmor profile,
  which gives it Home Assistant's maximum security rating, and pins the
  cloudflared release it fetches, verifying it against a recorded SHA-256 at
  build time.

## Troubleshooting

- Log shows `Unauthorized` or `token is invalid`: the token was copied wrong
  or belongs to a deleted tunnel. Paste it again from Zero Trust.
- Public hostname returns Cloudflare error 1033: the tunnel has no connected
  connector. Check the add-on is running and the log shows registered
  connections.
- Public hostname returns 502: the tunnel is up but the origin is unreachable
  from the add-on. Check the origin address in Zero Trust against the
  Addressing origins section.
- Nothing after `starting connector`: check the log for the four
  `Registered tunnel connection` lines. Set `loglevel` to `debug` for more.
