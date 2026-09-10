# Realmforge Gate — Independent Replacement

This directory contains newly authored Realmforge Gate replacement code.

It is intentionally separate from the covered interim implementation in `components/gate/`.

## Source rule

Code in this directory must be written from:

- public standards,
- Realmforge-owned architecture and requirements,
- Realmforge-owned black-box captures and fixtures,
- independently documented interoperability behavior.

Do not copy, translate, structurally port, import, or include implementation from the covered Gate tree.

## License status

No Realmforge-wide software license has been selected yet. Do not infer that this directory inherits the license of the covered interim Gate merely because both directories currently live in the same repository.

Any final licensing decision for newly authored Realmforge code requires owner approval and must be recorded explicitly.

## Current scope

The replacement currently establishes protocol-neutral semantic foundations for:

- client-build compatibility,
- realm identity and endpoints,
- identity/game-account projection,
- session lifecycle,
- emulator-independent world-auth handoff.

Protocol-specific adapters come later, after Realmforge-owned behavior fixtures exist.
