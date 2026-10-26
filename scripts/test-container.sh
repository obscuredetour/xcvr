#!/bin/sh
# Run the transmission crate's integration tests against a throwaway
# Transmission (linuxserver/transmission, pinned to the version the host runs)
# in Docker or Podman (OrbStack on macOS). The container is removed afterwards.
#
#   scripts/test-container.sh            all container tests (the client's, then the app's poller)
#   scripts/test-container.sh magnet     only tests whose names contain "magnet"
set -eu

IMAGE="${XCVR_TEST_IMAGE:-lscr.io/linuxserver/transmission:4.1.3}"
NAME="xcvr-test-transmission"
PORT="${XCVR_TEST_PORT:-19091}"
DOCKER="${DOCKER:-$(command -v docker || command -v podman)}"

cd "$(dirname "$0")/.."

"$DOCKER" rm -f "$NAME" >/dev/null 2>&1 || true
trap '"$DOCKER" rm -f "$NAME" >/dev/null 2>&1 || true' EXIT INT TERM

# The image first, a few tries: registries turn shared CI runners away now and
# then ("toomanyrequests"), for a moment.
if ! "$DOCKER" image inspect "$IMAGE" >/dev/null 2>&1; then
  for try in 1 2 3 4 5; do
    "$DOCKER" pull --quiet "$IMAGE" >/dev/null && break
    [ "$try" = 5 ] && { echo "Couldn't pull $IMAGE"; exit 1; }
    echo "Pulling $IMAGE failed; trying again in $((try * 10))s"
    sleep $((try * 10))
  done
fi

# USER/PASS turn on RPC authentication; WHITELIST lets the host's requests in
# (they arrive from the container network's gateway, not 127.0.0.1).
"$DOCKER" run -d --name "$NAME" \
  -p "127.0.0.1:$PORT:9091" \
  -e PUID="$(id -u)" -e PGID="$(id -g)" -e TZ=UTC \
  -e USER=xcvr -e PASS=xcvr \
  -e WHITELIST='*.*.*.*' \
  "$IMAGE" >/dev/null

printf 'Waiting for Transmission on port %s' "$PORT"
for _ in $(seq 1 60); do
  # Any HTTP answer (409 for the session id, 401 without credentials) means it's up.
  code="$(curl -s -o /dev/null -w '%{http_code}' "http://127.0.0.1:$PORT/transmission/rpc" || true)"
  case "$code" in
    409|401) echo " up."; break ;;
  esac
  printf '.'
  sleep 1
done
case "${code:-}" in
  409|401) ;;
  *) echo " no answer (last HTTP status: ${code:-none})."; "$DOCKER" logs "$NAME" | tail -20; exit 1 ;;
esac

# The client's tests, then the app's poller (its container tests are named container_*).
XCVR_TEST_PORT="$PORT" cargo test -p transmission --test container -- --ignored --test-threads=1 "$@"
XCVR_TEST_PORT="$PORT" cargo test -p xcvr -- --ignored --test-threads=1 container "$@"
