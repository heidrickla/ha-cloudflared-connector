#!/bin/sh
# Run the Cloudflare Tunnel connector for a dashboard-managed tunnel.
#
# The token is read from the add-on options and handed to cloudflared through
# the TUNNEL_TOKEN environment variable rather than on the command line, so it
# never appears in a process listing. Everything else about the tunnel (public
# hostnames, origins, Access policies) lives in the Cloudflare dashboard.
set -eu

OPTS=/data/options.json
TOKEN="$(jq -r '.tunnel_token // empty' "$OPTS")"
LOGLEVEL="$(jq -r '.loglevel // "info"' "$OPTS")"

if [ -z "$TOKEN" ]; then
  echo "[cloudflared-connector] tunnel_token is empty; set it in the add-on options." >&2
  sleep 30
  exit 1
fi

echo "[cloudflared-connector] starting connector, loglevel=${LOGLEVEL}"
export TUNNEL_TOKEN="$TOKEN"
exec /usr/local/bin/cloudflared --no-autoupdate --loglevel "$LOGLEVEL" tunnel run
