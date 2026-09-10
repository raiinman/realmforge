#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only
#
# Control the Realmforge dev Postgres 16 database via podman.
#
#   ./dev/db.sh up      start Postgres 16 (creates volume on first run)
#   ./dev/db.sh down    stop and remove the container (keeps the volume)
#   ./dev/db.sh reset   down, delete the volume, up (fresh database)
#   ./dev/db.sh logs    tail container logs
#   ./dev/db.sh psql    open a psql shell as the realmforge user
#
# Default connection: postgres://realmforge:realmforge@localhost:5432/realmforge

set -euo pipefail

IMAGE="docker.io/library/postgres:16"
NAME="realmforge-gate-db"
VOLUME="realmforge-gate-db-data"
HOST_PORT="5432"

cmd_up() {
	if podman container exists "$NAME" 2>/dev/null; then
		echo "$NAME already exists; start it with: podman start $NAME"
		exit 0
	fi
	echo "Starting $NAME (Postgres 16) on 127.0.0.1:${HOST_PORT}..."
	podman run -d --name "$NAME" \
		-e POSTGRES_USER=realmforge \
		-e POSTGRES_PASSWORD=realmforge \
		-e POSTGRES_DB=realmforge \
		-p "127.0.0.1:${HOST_PORT}:5432" \
		-v "${VOLUME}:/var/lib/postgresql/data" \
		"$IMAGE" >/dev/null
	echo "Started. Wait for readiness with: ./dev/db.sh logs"
	echo "Connect: postgres://realmforge:realmforge@localhost:${HOST_PORT}/realmforge"
}

cmd_down() {
	podman rm --force "$NAME" >/dev/null 2>&1 || true
	echo "Stopped $NAME (volume $VOLUME retained)."
}

cmd_reset() {
	cmd_down
	podman volume rm "$VOLUME" >/dev/null 2>&1 || true
	echo "Deleted volume $VOLUME."
	cmd_up
}

cmd_logs() {
	podman logs --tail=50 "$NAME"
}

cmd_psql() {
	podman exec -it "$NAME" psql -U realmforge -d realmforge
}

usage() {
	sed -n '3,16p' "$0" >&2
	exit 1
}

main() {
	local cmd="${1:-}"
	case "$cmd" in
	up) cmd_up ;;
	down) cmd_down ;;
	reset) cmd_reset ;;
	logs) cmd_logs ;;
	psql) cmd_psql ;;
	*) usage ;;
	esac
}

main "$@"
