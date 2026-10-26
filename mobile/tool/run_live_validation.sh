#!/usr/bin/env bash
# Exercise every backend route through the real Flutter HTTP client.
# Usage: mobile/tool/run_live_validation.sh [extra flutter test args]
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

if [[ -f "$ROOT/.env" ]]; then
  set -a
  # shellcheck disable=SC1091
  source "$ROOT/.env"
  set +a
fi

export LIVE_API_BASE_URL="${LIVE_API_BASE_URL:-http://localhost:8000}"
export LIVE_API_EMAIL="${LIVE_API_EMAIL:-${ADMIN_EMAIL:-}}"
export LIVE_API_PASSWORD="${LIVE_API_PASSWORD:-${ADMIN_PASSWORD:-}}"

if [[ -z "$LIVE_API_EMAIL" || -z "$LIVE_API_PASSWORD" ]]; then
  echo "Set LIVE_API_EMAIL/LIVE_API_PASSWORD or ADMIN_EMAIL/ADMIN_PASSWORD in .env." >&2
  exit 2
fi

cd "$ROOT/mobile"
exec flutter test test/live_api_test.dart "$@"
