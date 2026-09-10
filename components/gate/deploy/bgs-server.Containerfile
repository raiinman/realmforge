# SPDX-License-Identifier: AGPL-3.0-only
#
# Builder-pattern container for the Realmforge BGS transport (WS + raw TCP).
#
# Build:
#   podman build -f deploy/realmforge-gate-bgs-server.Containerfile -t realmforge/realmforge-gate-bgs-server:latest .
#
# Run:
#   podman run -d --name realmforge-gate-bgs \
#     -p 127.0.0.1:8119:8119 \
#     -p 127.0.0.1:1119:1119 \
#     -e DATABASE_URL=postgres://realmforge:realmforge@db:5432/realmforge \
#     -e BIND_ADDR=0.0.0.0:8119 \
#     -e TCP_BIND_ADDR=0.0.0.0:1119 \
#     realmforge/realmforge-gate-bgs-server:latest

# --- Builder stage ---------------------------------------------------------
FROM docker.io/library/rust:1.97-bookworm AS builder

WORKDIR /build
COPY . .

RUN cargo build --release --locked -p realmforge-gate-bgs-server

# --- Runtime stage ---------------------------------------------------------
FROM docker.io/library/debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --create-home realmforge

COPY --from=builder /build/target/release/realmforge-gate-bgs-server /usr/local/bin/realmforge-gate-bgs-server

USER realmforge
EXPOSE 8119 1119
ENTRYPOINT ["/usr/local/bin/realmforge-gate-bgs-server"]
