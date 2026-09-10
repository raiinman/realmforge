# Third-Party Components

Realmforge may vendor third-party source when doing so is permitted and materially accelerates the project.

This directory is intentionally separate from Realmforge-owned product source.

## Rules

Every vendored component must have, at minimum:

```text
<component>/
  UPSTREAM.md
  SOURCE_REVISION
  LICENSE.md (or the exact applicable license material)
  source/
```

`UPSTREAM.md` records origin, exact revision, import date, local modifications, integration boundary, and replacement status.

Do not move third-party files into first-party component directories merely to make them look native.

Do not remove applicable copyright/license notices from covered source.

Do not use third-party test/example keys as deployment secrets.

## Reconstruction boundary

Realmforge's independent replacement authority lives under:

```text
docs/reconstruction/
docs/research/
```

Vendored source is an interim implementation and regression reference. It is not the specification used to claim an independently reconstructed replacement.
