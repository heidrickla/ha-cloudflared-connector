#!/bin/sh
# Token goes to cloudflared via TUNNEL_TOKEN, not argv, to keep it out of the
# process list.
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
