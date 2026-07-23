#!/bin/sh
# PLENUM release container entrypoint (deploy unit, D2).
# Render injects PORT; the api binary reads BIND_ADDR (unchanged since P1) —
# translate one to the other and exec so the api is PID 1 and receives
# signals directly. No secrets here; everything arrives as env vars.
set -e
export BIND_ADDR="0.0.0.0:${PORT:-10000}"
exec /app/api
