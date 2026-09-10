#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
GATE = ROOT / "components" / "gate"

DIR_RENAMES = {
    GATE / "crates" / "tavern-account": GATE / "crates" / "realmforge-gate-account",
    GATE / "crates" / "tavern-bgs": GATE / "crates" / "realmforge-gate-bgs",
    GATE / "crates" / "tavern-core": GATE / "crates" / "realmforge-gate-core",
    GATE / "crates" / "tavern-db": GATE / "crates" / "realmforge-gate-db",
    GATE / "crates" / "tavern-oauth": GATE / "crates" / "realmforge-gate-oauth",
    GATE / "crates" / "tavern-observability": GATE / "crates" / "realmforge-gate-observability",
    GATE / "bin" / "account-server": GATE / "bin" / "realmforge-gate-account-server",
    GATE / "bin" / "bgs-server": GATE / "bin" / "realmforge-gate-bgs-server",
    GATE / "bin" / "oauth-server": GATE / "bin" / "realmforge-gate-oauth-server",
}

REPLACEMENTS = [
    ("tavern-observability", "realmforge-gate-observability"),
    ("tavern-account", "realmforge-gate-account"),
    ("tavern-oauth", "realmforge-gate-oauth"),
    ("tavern-core", "realmforge-gate-core"),
    ("tavern-db", "realmforge-gate-db"),
    ("tavern-bgs", "realmforge-gate-bgs"),
    ("tavern_observability", "realmforge_gate_observability"),
    ("tavern_account", "realmforge_gate_account"),
    ("tavern_oauth", "realmforge_gate_oauth"),
    ("tavern_core", "realmforge_gate_core"),
    ("tavern_db", "realmforge_gate_db"),
    ("tavern_bgs", "realmforge_gate_bgs"),
    ("account-server", "realmforge-gate-account-server"),
    ("bgs-server", "realmforge-gate-bgs-server"),
    ("oauth-server", "realmforge-gate-oauth-server"),
    ("Tavern", "Realmforge"),
    ("tavern", "realmforge"),
]

LEGAL_OR_PROVENANCE = {
    GATE / "LICENSE.md",
    GATE / "REALMFORGE_DERIVATION.md",
    GATE / ".realmforge-derived-from",
}

SKIP_BASENAMES = {"LICENSE", "LICENSE.txt", "COPYING", "NOTICE", "NOTICE.md"}


def rename_directories() -> None:
    for old, new in DIR_RENAMES.items():
        if old.exists():
            if new.exists():
                raise SystemExit(f"refusing to overwrite existing path: {new}")
            old.rename(new)


def should_skip(path: Path) -> bool:
    if path in LEGAL_OR_PROVENANCE:
        return True
    if path.name in SKIP_BASENAMES or path.name.startswith("LICENSE"):
        return True
    if "target" in path.parts or ".git" in path.parts:
        return True
    return False


def rewrite_text_files() -> None:
    for path in GATE.rglob("*"):
        if not path.is_file() or should_skip(path):
            continue
        try:
            text = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            continue
        new = text
        for old, replacement in REPLACEMENTS:
            new = new.replace(old, replacement)
        if new != text:
            path.write_text(new, encoding="utf-8")


def normalize_workspace_metadata() -> None:
    cargo = GATE / "Cargo.toml"
    text = cargo.read_text(encoding="utf-8")
    lines = []
    for line in text.splitlines():
        if line.startswith("authors = "):
            line = 'authors = ["Realmforge contributors"]'
        elif line.startswith("repository = "):
            line = 'repository = "https://github.com/raiinman/realmforge"'
        lines.append(line)
    cargo.write_text("\n".join(lines) + "\n", encoding="utf-8")


def replace_product_docs() -> None:
    (GATE / "README.md").write_text(
        """# Realmforge Gate\n\nRealmforge Gate is the retired-client compatibility and authentication edge for Realmforge.\n\nThis working tree is currently an AGPL-3.0-only covered derivative while inherited compatibility code remains. Product identity, package names, runtime configuration, logs, metrics, and user-facing surfaces are Realmforge-owned. Exact upstream provenance is quarantined in `REALMFORGE_DERIVATION.md` and the frozen baseline under `third_party/gate-upstream/`.\n\n## Responsibilities\n\n- retired-client authentication and compatibility transports,\n- session/ticket handling,\n- client-facing realm-list and realm-join projection,\n- typed Realmforge Gate runtime configuration,\n- compatibility evidence and regression fixtures.\n\n## Non-responsibilities\n\nGate does not own Realmforge canonical realm state, emulator-specific world state, or the administrator control plane. Those remain behind Core/Bridge/Forge boundaries.\n""",
        encoding="utf-8",
    )
    (GATE / "CHANGELOG.md").write_text(
        """# Realmforge Gate changelog\n\n## Rebuild foundation\n\n- established the covered Realmforge Gate working derivative,\n- introduced the Realmforge realm registry/projection boundary,\n- added typed Realmforge Gate configuration authority,\n- removed inherited product/package/service identity from the active Gate tree,\n- retained exact source provenance only in the legal/provenance boundary while inherited code remains.\n""",
        encoding="utf-8",
    )


def rename_exit_ledger() -> None:
    old = ROOT / "docs" / "reconstruction" / "TAVERN_EXIT_LEDGER.md"
    new = ROOT / "docs" / "reconstruction" / "UPSTREAM_EXIT_LEDGER.md"
    if old.exists() and not new.exists():
        old.rename(new)

    for rel in [
        "README.md",
        "docs/NEXT_CHAT_HANDOFF.md",
        "docs/ROADMAP.md",
        "docs/REALMFORGE_REBUILD_PLAN.md",
    ]:
        path = ROOT / rel
        if not path.exists():
            continue
        text = path.read_text(encoding="utf-8")
        text = text.replace("TAVERN_EXIT_LEDGER.md", "UPSTREAM_EXIT_LEDGER.md")
        path.write_text(text, encoding="utf-8")


def main() -> None:
    rename_directories()
    rewrite_text_files()
    normalize_workspace_metadata()
    replace_product_docs()
    rename_exit_ledger()


if __name__ == "__main__":
    main()
