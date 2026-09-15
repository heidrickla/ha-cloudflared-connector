# Cloudflared Connector

Runs the connector for a Cloudflare Tunnel that is managed in the Zero Trust
dashboard. The add-on holds only the connector token; hostnames, origins and
Access policies are configured in Cloudflare and pulled by the connector, so
exposing another service later needs no change here.

Options:

- `tunnel_token`: the connector token from Zero Trust, Networks, Tunnels.
- `loglevel`: cloudflared log level, default `info`.

The add-on runs on Home Assistant's internal network. Point each public
hostname's origin at Home Assistant (`http://homeassistant:8123`), at another
add-on by its hostname, or at a service on your LAN by address and port. See
the Documentation tab for details, origin addressing and troubleshooting.
