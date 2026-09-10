# SPDX-License-Identifier: AGPL-3.0-only
#
# Builder-pattern container for the Realmforge OAuth/OIDC provider.
#
# Build:
#   podman build -f deploy/realmforge-gate-oauth-server.Containerfile -t realmforge/realmforge-gate-oauth-server:latest .
#
# Run:
#   podman secret create realmforge-signing keys/signing.pem
#   podman run -d --name realmforge-gate-oauth --network realmforge-net \
#     -p 127.0.0.1:8081:8081 \
#     --secret realmforge-signing,type=mount,target=signing.pem \
#     -e DATABASE_URL=postgres://realmforge:realmforge@realmforge-gate-db:5432/realmforge \
#     -e BIND_ADDR=0.0.0.0:8081 \
#     -e ISSUER_URL=http://localhost:8081 \
#     -e SIGNING_KEY_PATH=/run/secrets/signing.pem \
#     realmforge/realmforge-gate-oauth-server:latest
#
# ISSUER_URL must match the externally-visible BIND_ADDR (oauth2c and other
# clients validate the discovery issuer against the URL they queried).
#
# Runtime deps: glibc only; sqlx uses rustls (no libpq/OpenSSL).

# --- Builder stage ---------------------------------------------------------
FROM docker.io/library/rust:1.97-bookworm AS builder

WORKDIR /build
COPY . .

RUN cargo build --release --locked -p realmforge-gate-oauth-server

# --- Runtime stage ---------------------------------------------------------
FROM docker.io/library/debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --create-home realmforge

COPY --from=builder /build/target/release/realmforge-gate-oauth-server /usr/local/bin/realmforge-gate-oauth-server

USER realmforge
EXPOSE 8081
ENTRYPOINT ["/usr/local/bin/realmforge-gate-oauth-server"]
