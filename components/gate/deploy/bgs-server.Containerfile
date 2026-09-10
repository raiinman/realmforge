# SPDX-License-Identifier: AGPL-3.0-only
#
# Builder-pattern container for the Tavern BGS transport (WS + raw TCP).
#
# Build:
#   podman build -f deploy/bgs-server.Containerfile -t tavern/bgs-server:latest .
#
# Run:
#   podman run -d --name tavern-bgs \
#     -p 127.0.0.1:8119:8119 \
#     -p 127.0.0.1:1119:1119 \
#     -e DATABASE_URL=postgres://tavern:tavern@db:5432/tavern \
#     -e BIND_ADDR=0.0.0.0:8119 \
#     -e TCP_BIND_ADDR=0.0.0.0:1119 \
#     tavern/bgs-server:latest

# --- Builder stage ---------------------------------------------------------
FROM docker.io/library/rust:1.97-bookworm AS builder

WORKDIR /build
COPY . .

RUN cargo build --release --locked -p bgs-server

# --- Runtime stage ---------------------------------------------------------
FROM docker.io/library/debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --create-home tavern

COPY --from=builder /build/target/release/bgs-server /usr/local/bin/bgs-server

USER tavern
EXPOSE 8119 1119
ENTRYPOINT ["/usr/local/bin/bgs-server"]
