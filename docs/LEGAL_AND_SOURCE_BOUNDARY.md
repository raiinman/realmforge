# Legal and Source Boundary

**Status:** D0 authority  
**Purpose:** Keep implementation lineage, third-party obligations, and future replacement work explicit.

This document is project engineering policy, not a substitute for legal advice.

## 0. Pinned interim-source fact

The currently researched compatibility implementation is `wowemulation-dev/tavern`.

At pinned revision `6f9158670ee7666bfae2be58b291dafcb45f12e7`, its workspace declares:

```text
license = "AGPL-3.0-only"
```

Realmforge must preserve the exact applicable upstream license/provenance while any copied or modified covered implementation remains. Earlier conversational shorthand describing it as "or later" is not repository authority.

## 1. Core rule

Realmforge may use third-party software when its license permits that use. Third-party obligations do not disappear because code is renamed, embedded, moved to another process, translated, or surrounded by Realmforge-owned code.

If Realmforge ships or operates a modified AGPL-covered network service, the corresponding AGPL obligations for that covered work remain applicable.

## 2. Product identity versus source lineage

Realmforge branding does not need to make a third-party component the public product identity.

However, legally required notices must remain where the applicable license requires them.

The project therefore separates:

- product branding,
- third-party notices,
- source licensing,
- protocol compatibility,
- implementation ownership.

## 3. Interim compatibility component policy

An AGPL-covered compatibility implementation may be used temporarily if it accelerates development.

It must be isolated behind a semantic service boundary so the rest of Realmforge does not depend on its internal structure.

Any copied or modified source from such a component remains covered according to that component's license until it is actually removed and independently replaced.

## 4. Exit strategy

The only valid way to call a component Realmforge-owned after starting from a third-party implementation is to remove the third-party-derived implementation and replace it with a genuinely independent implementation.

The accepted sequence is:

```text
third-party-derived implementation
          ↓
external behavior identified
          ↓
independent evidence captured
          ↓
source-independent specification frozen
          ↓
implementation written without source access
          ↓
interoperability verified
          ↓
derived implementation removed
          ↓
independent Realmforge component
```

Renaming files, changing most lines, or rewriting one language into another is not sufficient by itself.

## 5. Clean-room policy

For a component actively being reconstructed:

### Research lane may use

- public standards,
- independently generated packet captures,
- black-box behavior,
- target-client observations,
- public protocol research,
- legally obtained retired client binaries,
- independently licensed references,
- behavior-focused specifications.

### Implementation lane receives

- behavioral contracts,
- field tables,
- state machines,
- independent test vectors,
- capture-derived fixtures,
- acceptance tests,
- documented error semantics.

### Implementation lane should not use

- the third-party source being replaced,
- copied comments,
- copied internal decomposition,
- copied tests,
- copied migrations,
- copied UI source,
- line-by-line translations.

The complete reconstruction policy lives in `docs/reconstruction/TAVERN_EXIT_LEDGER.md`.

## 6. Third-party directory policy

If third-party source is later imported, keep it in an unmistakable location such as:

```text
third_party/
  <component>/
    LICENSE
    NOTICE
    UPSTREAM.md
    source...
```

or as a separately managed dependency/submodule where technically appropriate.

Do not mix third-party source into first-party directories without provenance metadata.

## 7. Provenance record

Every imported component should record:

```text
name
upstream URL
upstream revision/commit
import date
license identifier
license file path
local modifications
Realmforge integration boundary
replacement status
```

## 8. Realmforge-owned code

First-party Realmforge code should not be copied into an AGPL-derived source tree casually if doing so would make later separation ambiguous.

Prefer explicit service/API contracts and clear directories.

## 9. No-license assumption

Realmforge currently has no project-wide software license selected.

Do not add a license automatically.

Do not assume the repository's public visibility means third parties have permission to copy Realmforge-owned code beyond rights granted by applicable law or separately licensed components.

## 10. Game client and asset boundary

A protocol implementation is not the same thing as a right to redistribute a commercial game client or its assets.

Realmforge should be designed around users supplying their own legally obtained supported client installation unless a separately verified redistribution right exists.

Avoid embedding Blizzard trademarks, logos, artwork, audio, executable files, data files, or proprietary assets into Realmforge unless their use has been specifically reviewed.

## 11. Compatibility naming

Compatibility documentation may need to identify third-party products and protocol names accurately.

Product branding should make clear that Realmforge is an independent project and should not imply endorsement or affiliation.

## 12. Engineering acceptance rule

A component can be labeled `TAVERN-FREE` or otherwise independently implemented only when all of the following are true:

- no production binary links copied/modified upstream implementation code,
- no copied tests remain,
- no copied UI remains,
- no copied migrations remain,
- protocol behavior is documented from independent evidence,
- real-client compatibility passes the project's test matrix,
- an implementation engineer can maintain it without reopening the source being replaced.

Until then, keep provenance and applicable notices intact.
