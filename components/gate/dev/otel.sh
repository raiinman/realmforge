#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only
#
# Control the local OpenTelemetry Collector for Realmforge development.
#
#   ./dev/otel.sh up       start the collector (OTLP/HTTP in, Prometheus out)
#   ./dev/otel.sh down     stop and remove the collector container
#   ./dev/otel.sh logs     tail container logs
#   ./dev/otel.sh metrics  scrape the Prometheus endpoint
#
# Servers export OTLP to http://localhost:4318. Scrape metrics at
# http://localhost:8888/metrics.

set -euo pipefail

IMAGE="docker.io/otel/opentelemetry-collector-contrib:0.114.0"
NAME="realmforge-otel"
CONFIG_DIR="$(cd "$(dirname "$0")" && pwd)"

cmd_up() {
	if podman container exists "$NAME" 2>/dev/null; then
		echo "$NAME already exists; start it with: podman start $NAME"
		exit 0
	fi
	echo "Starting $NAME (OTLP :4318 in, Prometheus :8888 out)..."
	podman run -d --name "$NAME" \
		-p "127.0.0.1:4318:4318" \
		-p "127.0.0.1:19090:19090" \
		-v "${CONFIG_DIR}/otel-collector-config.yaml:/etc/otelcol/config.yaml:Z" \
		"$IMAGE" --config=/etc/otelcol/config.yaml >/dev/null
	echo "Started. Scrape metrics with: ./dev/otel.sh metrics"
}

cmd_down() {
	podman rm --force "$NAME" >/dev/null 2>&1 || true
	echo "Stopped $NAME."
}

cmd_logs() {
	podman logs --tail=50 "$NAME"
}

cmd_metrics() {
	curl -s "http://localhost:19090/metrics"
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
	logs) cmd_logs ;;
	metrics) cmd_metrics ;;
	*) usage ;;
	esac
}

main "$@"
