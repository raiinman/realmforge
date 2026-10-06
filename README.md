<!-- project-centered:start -->
<div align="center">

<a name="readme-top"></a>
<h1 align="center">Realmforge</h1>

<!-- project-header:start -->
<p align="center"><img src="readme-banner.png" alt="realmforge — original decorative project artwork" width="100%"></p>
<!-- project-header:end -->

<!-- project-badges:start -->
<p align="center"><a href="https://github.com/raiinman/realmforge"><img src="https://img.shields.io/badge/phase-D0_evidence_campaign-BC7838?logo=github&amp;logoColor=white" alt="phase: D0 evidence campaign"></a> <a href="https://github.com/raiinman/realmforge"><img src="https://img.shields.io/badge/access-public-BC7838?logo=github&amp;logoColor=white" alt="access: public"></a> <a href="#readme-index"><img src="https://img.shields.io/badge/docs-explore_the_index-BC7838?logo=readthedocs&amp;logoColor=white" alt="docs: explore the index"></a></p>
<!-- project-badges:end -->

<!-- project-live-badges:start -->
<p align="center"><a href="https://github.com/raiinman/realmforge/commits/main"><img src="https://img.shields.io/github/last-commit/raiinman/realmforge?color=BC7838&amp;logo=git&amp;logoColor=white" alt="GitHub last commit"></a> <a href="https://github.com/raiinman/realmforge/issues"><img src="https://img.shields.io/github/issues/raiinman/realmforge?color=BC7838&amp;logo=github&amp;logoColor=white" alt="GitHub open issues"></a> <a href="https://github.com/raiinman/realmforge/stargazers"><img src="https://badgen.net/github/stars/raiinman/realmforge?icon=github&amp;color=BC7838" alt="GitHub stars"></a></p>
<!-- project-live-badges:end -->

<!-- project-index:start -->
<a name="readme-index"></a>
<h3 align="center">✦ Explore this project</h3>
<table align="center"><tbody><tr><td align="center"><a href="#readme-overview"><strong>Overview</strong></a></td><td align="center"><a href="#readme-status"><strong>Status</strong></a></td></tr><tr><td align="center"><a href="#readme-product-shape"><strong>Product shape</strong></a></td><td align="center"><a href="#readme-core-rule"><strong>Core rule</strong></a></td></tr><tr><td align="center"><a href="#readme-repository-authority"><strong>Repository authority</strong></a></td><td align="center"><a href="#readme-current-target-architecture"><strong>Current target architecture</strong></a></td></tr><tr><td align="center"><a href="#readme-licensing"><strong>Licensing</strong></a></td><td align="center"><a href="#readme-project-identity"><strong>Project identity</strong></a></td></tr></tbody></table>
<h4 align="center">Project shortcuts</h4>
<table align="center"><tbody><tr><td align="center"><a href="docs/PROJECT_CHARTER.md"><strong>PROJECT CHARTER</strong></a></td><td align="center"><a href="docs/ARCHITECTURE.md"><strong>ARCHITECTURE</strong></a></td></tr><tr><td align="center"><a href="docs/ROADMAP.md"><strong>ROADMAP</strong></a></td><td align="center"><a href="docs/reconstruction/TAVERN_EXIT_LEDGER.md"><strong>reconstruction/TAVERN EXIT LEDGER</strong></a></td></tr></tbody></table>
<!-- project-index:end -->

<a name="readme-overview"></a>
<h2 align="center">Overview</h2>

**Self-hosted realm infrastructure for preserved MMO clients.**

Realmforge is a control plane, launcher, client manager, realm orchestrator, emulator-adapter layer, and compatibility-services platform intended to make running private preserved game realms feel like operating a modern self-hosted service instead of assembling a pile of unrelated tools by hand.

The first compatibility target is retired World of Warcraft Classic client infrastructure. The long-term architecture is deliberately broader than one game, one emulator, or one authentication implementation.

<p align="center"><a href="#readme-index">↑ Back to index</a></p>

<a name="readme-status"></a>
## Status


**Phase:** D0 — project authority / evidence campaign

Production implementation is **not yet authorized as settled architecture**. The immediate job is to document the system tightly enough that implementation agents do not fill unresolved behavior with assumptions.

The project currently has two parallel goals:

<table align="center"><tbody><tr><td align="center">1</td><td align="center">Use an existing compatible authentication/BGS implementation as an interim subsystem where useful.</td></tr><tr><td align="center">2</td><td align="center">Build a source-independent reconstruction authority so every inherited component can eventually be replaced by Realmforge-owned code without needing to reopen the upstream implementation.</td></tr></tbody></table>


See [`docs/reconstruction/TAVERN_EXIT_LEDGER.md`](docs/reconstruction/TAVERN_EXIT_LEDGER.md) for the current reconstruction authority.

<p align="center"><a href="#readme-index">↑ Back to index</a></p>

<a name="readme-product-shape"></a>
## Product shape


<table align="center"><tbody><tr><td align="left"><pre><code>Realmforge
│
├── Launcher / Client Manager
├── Control Center
├── Realm Orchestrator
├── Realm Registry
├── Emulator Adapter Layer
├── Deployment / Updates
├── Backup / Restore
├── Observability
├── Community / Invite Layer
│
└── Compatibility Gateway
    ├── Identity / credentials
    ├── OAuth / OIDC
    ├── Browser login
    ├── Game-client login
    ├── BGS transport
    ├── Session state
    ├── Realm-list / realm-join handoff
    └── Desktop-app SSO</code></pre></td></tr></tbody></table>


<p align="center"><a href="#readme-index">↑ Back to index</a></p>

<a name="readme-core-rule"></a>
## Core rule


Realmforge distinguishes **behavioral compatibility** from **implementation lineage**.

A component is not considered independently owned merely because it was renamed, moved, translated to another language, or heavily modified. The replacement path is:

<table align="center"><tbody><tr><td align="left"><pre><code>inherited implementation
        ↓
observable behavior documented
        ↓
independent evidence captured
        ↓
source-independent specification frozen
        ↓
independent implementation written
        ↓
real-client interoperability verified
        ↓
old implementation removed
        ↓
Realmforge-owned component</code></pre></td></tr></tbody></table>


If an implementation engineer must reopen upstream source to finish a replacement, the reconstruction package is incomplete.

<p align="center"><a href="#readme-index">↑ Back to index</a></p>

<a name="readme-repository-authority"></a>
## Repository authority


Read these before changing architecture:

<table align="center"><tbody><tr><td align="center"><a href="docs/PROJECT_CHARTER.md"><code>docs/PROJECT_CHARTER.md</code></a></td></tr><tr><td align="center"><a href="docs/ARCHITECTURE.md"><code>docs/ARCHITECTURE.md</code></a></td></tr><tr><td align="center"><a href="docs/ROADMAP.md"><code>docs/ROADMAP.md</code></a></td></tr><tr><td align="center"><a href="docs/DECISIONS.md"><code>docs/DECISIONS.md</code></a></td></tr><tr><td align="center"><a href="docs/reconstruction/TAVERN_EXIT_LEDGER.md"><code>docs/reconstruction/TAVERN_EXIT_LEDGER.md</code></a></td></tr><tr><td align="center"><a href="docs/research/COMPATIBILITY_RESEARCH_CAMPAIGN.md"><code>docs/research/COMPATIBILITY_RESEARCH_CAMPAIGN.md</code></a></td></tr><tr><td align="center"><a href="docs/LEGAL_AND_SOURCE_BOUNDARY.md"><code>docs/LEGAL_AND_SOURCE_BOUNDARY.md</code></a></td></tr><tr><td align="center"><a href="docs/NEXT_CHAT_HANDOFF.md"><code>docs/NEXT_CHAT_HANDOFF.md</code></a></td></tr></tbody></table>


<p align="center"><a href="#readme-index">↑ Back to index</a></p>

<a name="readme-current-target-architecture"></a>
## Current target architecture


Realmforge should make the common case approximately:

<table align="center"><tbody><tr><td align="left"><pre><code>Install Realmforge
      ↓
Open Control Center
      ↓
Create administrator
      ↓
Detect/import supported retired client
      ↓
Choose/install compatible emulator adapter
      ↓
Create realm
      ↓
Invite users
      ↓
Play</code></pre></td></tr></tbody></table>


The platform should own the operational experience even when individual compatibility or emulator components are third-party software.

<p align="center"><a href="#readme-index">↑ Back to index</a></p>

<a name="readme-licensing"></a>
## Licensing


No Realmforge-wide software license has been selected yet. Do not assume that absence of a license grants permission to reuse Realmforge code.

Third-party components retain their own licenses and obligations. See [`docs/LEGAL_AND_SOURCE_BOUNDARY.md`](docs/LEGAL_AND_SOURCE_BOUNDARY.md).

<p align="center"><a href="#readme-index">↑ Back to index</a></p>

<a name="readme-project-identity"></a>
## Project identity


**Realmforge** is the product name.

Working subsystem names:

<table align="center"><tbody><tr><td align="center"><strong>Gate</strong> — protocol/authentication compatibility gateway</td></tr><tr><td align="center"><strong>Core</strong> — control plane and canonical state</td></tr><tr><td align="center"><strong>Forge</strong> — realm lifecycle/orchestration</td></tr><tr><td align="center"><strong>Client</strong> — launcher and local client manager</td></tr><tr><td align="center"><strong>Console</strong> — administrator UI</td></tr><tr><td align="center"><strong>Bridge</strong> — emulator adapters</td></tr></tbody></table>


These names are working architecture labels, not locked branding.

<p align="center"><a href="#readme-index">↑ Back to index</a> · <a href="#readme-top">Back to top ↑</a></p>

</div>
<!-- project-centered:end -->
