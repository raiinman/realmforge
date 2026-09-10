# SPDX-License-Identifier: AGPL-3.0-only
#
# Builder-pattern container for the Realmforge account server (web + API + login).
#
# Build:
#   podman build -f deploy/realmforge-gate-account-server.Containerfile -t realmforge/realmforge-gate-account-server:latest .
#
# Run (signing key as a podman secret — bind mounts of the demo key fail
# because the container user (uid 10001) cannot read the 0600 host file):
#   podman secret create realmforge-signing keys/signing.pem
#   podman run -d --name realmforge-gate-account --network realmforge-net \
#     -p 127.0.0.1:8080:8080 \
#     --secret realmforge-signing,type=mount,target=signing.pem \
#     -e DATABASE_URL=postgres://realmforge:realmforge@realmforge-gate-db:5432/realmforge \
#     -e BIND_ADDR=0.0.0.0:8080 \
#     -e SIGNING_KEY_PATH=/run/secrets/signing.pem \
#     -e SMTP_HOST=realmforge-mail -e SMTP_PORT=1025 \
#     realmforge/realmforge-gate-account-server:latest
#
# Runtime deps: glibc only. The Postgres driver (sqlx, runtime-tokio-rustls)
# is pure Rust — no libpq, no OpenSSL. ca-certificates covers outbound TLS
# (Postgres over TLS + SMTP) via the OS store rustls reads.

# --- Builder stage ---------------------------------------------------------
FROM docker.io/library/rust:1.97-bookworm AS builder

# The workspace ships the sqlx offline cache (.sqlx/, committed) so the
# query macros compile without a live DATABASE_URL.
WORKDIR /build
COPY . .

RUN cargo build --release --locked -p realmforge-gate-account-server

# --- Runtime stage ---------------------------------------------------------
FROM docker.io/library/debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --create-home realmforge

COPY --from=builder /build/target/release/realmforge-gate-account-server /usr/local/bin/realmforge-gate-account-server

USER realmforge
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/realmforge-gate-account-server"]
