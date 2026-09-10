# Signing Keys

This directory contains demo RSA signing keys for the Tavern OAuth provider.

## `signing.pem`

A 2048-bit PKCS#8 RSA private key used to sign RS256 JWTs. This is a
**demo key** — it is the same key used in the test suite and is safe to
commit because it protects nothing. **Do not use in production.**

To generate a production key:

```bash
openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 -out keys/signing.pem
```

Then set `SIGNING_KEY_PATH=keys/signing.pem` (or an absolute path) when
running `oauth-server`.
