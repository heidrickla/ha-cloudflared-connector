# Cloudflared Connector

Connector for a Cloudflare Tunnel configured in the Zero Trust dashboard.
Hostnames, origins and Access policies are set in Cloudflare; this add-on holds
only the connector token.

Set `tunnel_token` and start. Uses Home Assistant's internal network, so point
origins at `http://homeassistant:8123`, another add-on's hostname, or a LAN
address and port.

Setup, origins and troubleshooting: Documentation tab.
