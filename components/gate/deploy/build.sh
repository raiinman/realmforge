#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only
#
# Build the three Realmforge service images with the builder pattern.
#
#   ./deploy/build.sh            build all three (tag: latest)
#   ./deploy/build.sh account    build one service only
#
# Requires podman and a build context of the repo root.

set -euo pipefail

cd "$(dirname "$0")/.."

TAG="${TAG:-latest}"
SERVICES=(account oauth bgs)

build_one() {
	local name="$1"
	echo "==> building realmforge/${name}-server:${TAG}"
	podman build \
		-f "deploy/${name}-server.Containerfile" \
		-t "realmforge/${name}-server:${TAG}" \
		.
}

if [ $# -eq 0 ]; then
	for s in "${SERVICES[@]}"; do
		build_one "$s"
	done
else
	for s in "$@"; do
		case " ${SERVICES[*]} " in
		*" $s "*) build_one "$s" ;;
		*) echo "unknown service: $s (valid: ${SERVICES[*]})" >&2; exit 1 ;;
		esac
	done
fi

echo "done"
