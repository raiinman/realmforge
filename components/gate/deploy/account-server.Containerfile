# SPDX-License-Identifier: AGPL-3.0-only
#
# Builder-pattern container for the Tavern account server (web + API + login).
#
# Build:
#   podman build -f deploy/account-server.Containerfile -t tavern/account-server:latest .
#
# Run (signing key as a podman secret — bind mounts of the demo key fail
# because the container user (uid 10001) cannot read the 0600 host file):
#   podman secret create tavern-signing keys/signing.pem
#   podman run -d --name tavern-account --network tavern-net \
#     -p 127.0.0.1:8080:8080 \
#     --secret tavern-signing,type=mount,target=signing.pem \
#     -e DATABASE_URL=postgres://tavern:tavern@tavern-db:5432/tavern \
#     -e BIND_ADDR=0.0.0.0:8080 \
#     -e SIGNING_KEY_PATH=/run/secrets/signing.pem \
#     -e SMTP_HOST=tavern-mail -e SMTP_PORT=1025 \
#     tavern/account-server:latest
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

RUN cargo build --release --locked -p account-server

# --- Runtime stage ---------------------------------------------------------
FROM docker.io/library/debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --create-home tavern

COPY --from=builder /build/target/release/account-server /usr/local/bin/account-server

USER tavern
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/account-server"]
