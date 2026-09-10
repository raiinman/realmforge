# Realmforge Container Deployment

Builder-pattern containers for the three services. All static assets are
compiled into the binaries (Askama templates, SPA JS/CSS, srp6a.js), so the
runtime image is a slim Debian with just the binary, CA certificates, and a
non-root user.

## Images

| Image | Builds | Ports |
|---|---|---|
| `realmforge/realmforge-gate-account-server` | `deploy/realmforge-gate-account-server.Containerfile` | 8080 (HTTP) |
| `realmforge/realmforge-gate-oauth-server` | `deploy/realmforge-gate-oauth-server.Containerfile` | 8081 (HTTP) |
| `realmforge/realmforge-gate-bgs-server` | `deploy/realmforge-gate-bgs-server.Containerfile` | 8119 (WS), 1119 (TCP) |

Build all three (or one) with `deploy/build.sh`:

```bash
./deploy/build.sh              # all three, tag latest
./deploy/build.sh account      # just realmforge-gate-account-server
```

Requires podman. The builder stage compiles with `cargo build --release
--locked`; the workspace ships the sqlx offline cache (`.sqlx/`, committed)
so no live `DATABASE_URL` is needed at build time.

## Runtime requirements

- **glibc only.** The Postgres driver (sqlx, `runtime-tokio-rustls`) is pure
  Rust — no libpq, no OpenSSL. No Postgres client libraries or dev headers
  in either stage.
- **`ca-certificates`** in the runtime stage for outbound TLS (Postgres over
  TLS, SMTP) — rustls reads the OS store.
- **Non-root user** (`realmforge`, uid 10001) in the runtime stage.

## Network

Containers on the same custom bridge resolve each other by container name:

```bash
podman network create --driver bridge realmforge-net
podman run -d --name realmforge-gate-db --network realmforge-net \
  -e POSTGRES_USER=realmforge -e POSTGRES_PASSWORD=realmforge -e POSTGRES_DB=realmforge \
  -v realmforge-gate-db-data:/var/lib/postgresql/data \
  docker.io/library/postgres:16
```

The services reach the DB as `postgres://realmforge:realmforge@realmforge-gate-db:5432/realmforge`.

## Mail (SMTP)

The realmforge-gate-account-server sends verification mail over SMTP. Point it at a mail
catcher (or real relay) **on the same network**, by container name:

```bash
podman run -d --name realmforge-mail --network realmforge-net \
  -p 127.0.0.1:1025:1025 -p 127.0.0.1:1080:1080 \
  docker.io/marlonb/mailcrab:latest

# realmforge-gate-account-server:
-e SMTP_HOST=realmforge-mail -e SMTP_PORT=1025
```

The realmforge-gate-account-server must be started with `SMTP_HOST`/`SMTP_PORT` (defaults
are `localhost:1025`, which inside a container is itself). The mail catcher
must be on `realmforge-net` from the start — attaching it later is not possible
with the rootless 'pasta' network backend (`podman network connect` fails).

## Signing key as a secret

The runtime user cannot read a host bind-mount of `keys/signing.pem`
(0600, owned by the host user). Use a podman secret:

```bash
podman secret create realmforge-signing keys/signing.pem
podman run -d --name realmforge-gate-account --network realmforge-net \
  -p 127.0.0.1:8080:8080 \
  --secret realmforge-signing,type=mount,target=signing.pem \
  -e DATABASE_URL=postgres://realmforge:realmforge@realmforge-gate-db:5432/realmforge \
  -e BIND_ADDR=0.0.0.0:8080 \
  -e SIGNING_KEY_PATH=/run/secrets/signing.pem \
  -e SMTP_HOST=realmforge-mail -e SMTP_PORT=1025 \
  realmforge/realmforge-gate-account-server:latest
```

`SIGNING_KEY_PATH` points at the mounted secret. For production, generate a
key with `openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048`
(see `keys/README.md`) and load it as a secret — never bake it into the
image.

## Configuration

All config is environment-based (`realmforge_gate_core::Config::from_env`):

- `DATABASE_URL` (required), `BIND_ADDR`, `ISSUER_URL` (oauth),
  `SIGNING_KEY_PATH`, `SMTP_HOST`/`SMTP_PORT`/`SMTP_FROM` (account),
  `REGION`, DB pool vars.
- `TOKIO_WORKER_THREADS`, `MAX_CONCURRENT_REQUESTS` (tuning).
- bgs: `MAX_BGS_LOGINS`, `TCP_BIND_ADDR`.
