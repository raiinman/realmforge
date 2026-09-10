#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only
#
# Control the Tavern dev mail catcher (mailcrab) via podman.
# Catches all outbound SMTP so welcome/verification emails can be
# inspected in a browser. SMTP on 127.0.0.1:1025 (the account-server
# default), web UI on http://127.0.0.1:1080.
#
#   ./dev/mail.sh up      start mailcrab
#   ./dev/mail.sh down    stop and remove the container
#   ./dev/mail.sh logs    tail container logs

set -euo pipefail

IMAGE="docker.io/marlonb/mailcrab:latest"
NAME="tavern-mail"
SMTP_PORT="1025"
UI_PORT="1080"

cmd_up() {
	if podman container exists "$NAME" 2>/dev/null; then
		echo "$NAME already exists; start it with: podman start $NAME"
		exit 0
	fi
	echo "Starting $NAME (mailcrab) — SMTP 127.0.0.1:${SMTP_PORT}, UI http://127.0.0.1:${UI_PORT}"
	podman run -d --name "$NAME" \
		-p "127.0.0.1:${SMTP_PORT}:1025" \
		-p "127.0.0.1:${UI_PORT}:1080" \
		"$IMAGE"
	echo "Mail UI: http://127.0.0.1:${UI_PORT}"
}

cmd_down() {
	if podman container exists "$NAME" 2>/dev/null; then
		podman rm -f "$NAME" >/dev/null
		echo "Removed $NAME."
	else
		echo "$NAME is not running."
	fi
}

cmd_logs() {
	podman logs --tail=50 "$NAME"
}

main() {
	case "${1:-}" in
	up) cmd_up ;;
	down) cmd_down ;;
	logs) cmd_logs ;;
	*)
		echo "Usage: $0 {up|down|logs}"
		exit 1
		;;
	esac
}

main "$@"
