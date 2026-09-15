# Cloudflared Connector

Runs the connector for a Cloudflare Tunnel configured in the Zero Trust
dashboard. Hostnames, origins and Access policies are set in Cloudflare; the
add-on holds only the connector token.

## Setup

1. Zero Trust > Networks > Tunnels. Create a tunnel with the Cloudflare
   (dashboard) configuration option. Copy the connector token.
2. Add a public hostname to the tunnel, origin per the table below.
3. Add a Cloudflare Access application for that hostname. Without one the
   service is on the open internet.
4. Set `tunnel_token` here and start the add-on.

Up when the log shows `Registered tunnel connection` four times and Zero Trust
reports the tunnel Healthy. The install command Cloudflare displays is not
needed; this add-on replaces it.

## Options

| Option | Required | Default | Values |
|---|---|---|---|
| `tunnel_token` | yes | | Connector token from Zero Trust |
| `loglevel` | no | `info` | `debug`, `info`, `warn`, `error` |

## Origins

The add-on uses Home Assistant's internal network, not host networking.

| Target | Origin |
|---|---|
| Home Assistant | `http://homeassistant:8123` |
| Another add-on | `http://<slug with dashes>:<container port>` |
| LAN service | `http://<address>:<port>` |

## Security

- Token reaches cloudflared through `TUNNEL_TOKEN`, not the command line.
- No host networking. AppArmor profile. Security rating 8.
- cloudflared pinned to one release, SHA-256 verified per architecture at
  build.
- To rotate a leaked token: recreate the tunnel token in Zero Trust, then
  update the option.

## Troubleshooting

| Symptom | Cause |
|---|---|
| `Unauthorized`, invalid token | Token mistyped, or its tunnel was deleted |
| Cloudflare error 1033 | No connector connected; check the add-on is running |
| 502 | Origin unreachable from the add-on; see Origins |
| No registered connections | Set `loglevel` to `debug` and re-read the log |
