# Cloudflared Connector

Runs the connector for a Cloudflare Tunnel configured in the Zero Trust dashboard. Hostnames, origins and Access policies are set in Cloudflare; the add-on holds only the connector token.

## Setup

1. Zero Trust > Networks > Tunnels. Create a tunnel with the Cloudflare (dashboard) configuration option. Copy the connector token.
2. Add a public hostname to the tunnel, origin per the table below.
3. Add a Cloudflare Access application for that hostname. Without one the service is on the open internet.
4. Set `tunnel_token` here and start the add-on.

Up when the Cloudflared panel shows four edge connections and Zero Trust reports the tunnel Healthy. The install command Cloudflare displays is not needed; this add-on replaces it.

## Panel

Cloudflared in the sidebar, or Open Web UI on this page. Only Home Assistant's ingress proxy reaches it.

| Tab | Shows |
|---|---|
| Overview | State; each edge connection with its Cloudflare location, protocol, round trip and bytes; requests, origin errors and responses by status code; cloudflared version, connector ID, configuration version, log level, restarts |
| Hostnames | Public hostnames, paths and services from the Zero Trust dashboard, as cloudflared is running them; the catch-all service last |
| Log | The last 300 lines cloudflared printed, filtered by level |

Restart cloudflared (Overview, Connector card) asks for a second click. It drops every connection through the tunnel for a few seconds.

![Overview](https://raw.githubusercontent.com/heidrickla/ha-cloudflared-connector/main/docs/screenshots/overview.png)

![Hostnames](https://raw.githubusercontent.com/heidrickla/ha-cloudflared-connector/main/docs/screenshots/hostnames.png)

![Log](https://raw.githubusercontent.com/heidrickla/ha-cloudflared-connector/main/docs/screenshots/log.png)

Hostnames are edited in Zero Trust, not here.

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

- Token reaches cloudflared through `TUNNEL_TOKEN`, not the command line. The panel never shows it.
- The connector runs cloudflared as its child and starts it again with backoff, 1 s up to 60 s, if it exits.
- cloudflared's metrics and readiness endpoints listen on `127.0.0.1:20241` inside the container only.
- No host networking. AppArmor profile. Security rating 8.
- cloudflared pinned to one release, SHA-256 verified per architecture at build.
- To rotate a leaked token: recreate the tunnel token in Zero Trust, then update the option.

## Troubleshooting

| Symptom | Cause |
|---|---|
| `Unauthorized`, invalid token | Token mistyped, or its tunnel was deleted |
| Cloudflare error 1033 | No connector connected; check the add-on is running |
| 502 | Origin unreachable from the add-on; see Origins |
| No registered connections | Set `loglevel` to `debug` and re-read the Log tab |
