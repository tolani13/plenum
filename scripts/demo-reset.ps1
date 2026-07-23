# PLENUM — one-line demo reset (R11).
# Reseeds the world: truncate + regenerate (identical every run, PRNG seed
# 20260717), then — inside the same seed binary — refresh_rollups(), a
# post-load ANALYZE, and generate_signals(). Config tables (discount_policy,
# signal_policy, territory_states) are never truncated.
#
# Note: API sessions live in process memory, so open browser sessions need a
# fresh login after a reset (by design — recorded demo-phase limit).

$repo = Split-Path -Parent $PSScriptRoot
Push-Location $repo
try {
    cargo run --bin seed
} finally {
    Pop-Location
}
