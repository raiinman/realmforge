# Realmforge Architecture Authority

**Status:** D0 target architecture  
**Purpose:** Prevent accidental monolith design and keep third-party compatibility code replaceable.

## 1. System overview

```text
                           ┌─────────────────────────┐
                           │      Realmforge UI      │
                           │ Console / Launcher UX   │
                           └────────────┬────────────┘
                                        │
                           ┌────────────▼────────────┐
                           │          Core           │
                           │ canonical control plane │
                           └───────┬─────┬─────┬─────┘
                                   │     │     │
                     ┌─────────────┘     │     └─────────────┐
                     ▼                   ▼                   ▼
              ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
              │    Forge    │     │    Gate     │     │   Client    │
              │ realm ops   │     │ compat/auth │     │ local agent │
              └──────┬──────┘     └──────┬──────┘     └──────┬──────┘
                     │                   │                   │
                     ▼                   ▼                   ▼
              ┌─────────────┐     retired game         local installs
              │   Bridge    │        clients           and launcher
              │ adapters    │
              └──────┬──────┘
                     │
          ┌──────────┼──────────┐
          ▼          ▼          ▼
      emulator A emulator B emulator C
```

## 2. Core

Core is the product authority and must remain independent from a single emulator or wire protocol.

Core owns canonical resources such as:

```text
User
Administrator
Invite
ClientInstall
ClientProfile
Realm
RealmRuntime
RealmTemplate
EmulatorInstallation
Adapter
Backup
Deployment
HealthState
AuditEvent
SecretReference
CompatibilityProfile
```

Core must never treat raw BGS protobufs, emulator-specific SQL rows, or launcher config files as canonical product state.

## 3. Gate

Gate is the compatibility boundary between retired clients and Realmforge identity/realm state.

Responsibilities may include:

- protocol-specific authentication,
- browser login compatibility,
- OAuth/OIDC compatibility,
- BGS transport and RPC,
- game-account projection,
- session lifecycle,
- realm-list translation,
- realm-join ticketing/handoff,
- desktop-app SSO compatibility.

Gate may initially contain or wrap third-party code. That fact must not leak into Core's data model.

### Gate invariant

Core asks Gate to perform semantic operations such as:

```text
AuthenticateIdentity
ListCompatibleRealms
CreateClientSession
IssueRealmJoin
DisconnectSession
```

Core should not ask Gate to expose arbitrary upstream-internal functions.

## 4. Forge

Forge owns lifecycle operations for realm runtimes:

- create,
- validate,
- configure,
- start,
- stop,
- restart,
- health,
- logs,
- metrics,
- backup,
- restore,
- upgrade,
- rollback,
- destroy.

Forge works through Bridge adapters rather than hard-coded emulator logic.

## 5. Bridge

Bridge is the adapter contract for emulator/server implementations.

Minimum conceptual contract:

```text
detect()
install()
validate()
configure()
registerRealm()
start()
stop()
restart()
status()
logs()
metrics()
backup()
restore()
upgrade()
rollback()
```

Adapters may expose richer optional capability sets, but Core must be able to reason about missing capabilities.

Example capability flags:

```text
supportsHotReload
supportsOnlineBackup
supportsCharacterCount
supportsPopulationTelemetry
supportsDynamicRealmRegistration
supportsGracefulShutdown
supportsRollingUpgrade
```

## 6. Client

The local Realmforge Client application owns user-machine concerns:

- discover installed compatible clients,
- identify exact build,
- hash/validate relevant files,
- store named launch profiles,
- configure endpoints safely,
- back up original client settings,
- restore settings,
- manage launch arguments,
- test Realmforge connectivity,
- launch selected profile,
- expose actionable diagnostics.

### Client security boundary

If a localhost agent exists, it must:

- bind locally by default,
- authenticate requests,
- reject arbitrary shell execution,
- use typed operations,
- validate paths,
- have explicit origin/CORS policy,
- separate privileged operations from ordinary launch operations.

## 7. Console

Console is the administrator experience.

Primary areas:

- Overview
- Realms
- Players/Accounts
- Clients
- Emulator installations
- Backups
- Deployments
- Logs
- Health
- Invites
- Settings
- Security

Console must show failures in operational language rather than raw stack traces as the normal UX.

Example:

```text
Realm failed to start

Cause: database migration 20260909_03 did not complete.
Last healthy backup: 18 minutes ago.

[View migration log] [Restore backup] [Retry]
```

## 8. Realm registry

Core owns one normalized realm model.

Minimum target shape:

```text
Realm
- id
- displayName
- game
- expansion/contentFamily
- supportedClientBuilds
- emulatorAdapter
- runtimeHost
- publicAddress
- gamePort
- visibility
- maintenanceState
- populationState
- healthState
- authPolicy
- tags
```

Protocol-specific representations are projections of this object.

## 9. Identity

Long-term Realmforge identity should be ours.

Compatibility requirements may force protocol-specific credentials or identifiers. Keep those in separate records rather than making them the user's primary identity model.

Conceptually:

```text
RealmforgeUser
  ├── ProductCredential
  ├── CompatibilityCredential(s)
  ├── GameAccountProjection(s)
  ├── OAuthSubject(s)
  └── Membership/Entitlements
```

Do not force product passwords to use historical game protocol password storage when an adapter can maintain a protocol-specific verifier separately.

## 10. Persistence

Core should own its own migration history.

Third-party components may temporarily own their own schemas. Cross-schema reads should be minimized; preferred integration is through explicit service contracts.

Avoid making Realmforge business logic depend directly on a third-party schema because it makes replacement harder.

## 11. Event model

Prefer explicit domain events for state changes that multiple components care about:

```text
RealmCreated
RealmStarted
RealmStopped
RealmHealthChanged
RealmBuildCompatibilityChanged
ClientDetected
ClientProfileCreated
AccountCreated
SessionStarted
SessionEnded
BackupCompleted
DeploymentFailed
```

This does not mandate a distributed message broker in D0. An in-process or database-backed event mechanism may be sufficient initially.

## 12. API boundary

Realmforge APIs should be versioned around product semantics.

Example:

```text
POST /api/v1/realms
POST /api/v1/realms/{id}/start
GET  /api/v1/realms/{id}/health
POST /api/v1/client-profiles
POST /api/v1/backups
```

Avoid exposing compatibility protocol objects directly through the admin API.

## 13. Deployment profiles

### Single-node

```text
Realmforge
Postgres
Gate
one or more emulator processes
reverse proxy
```

Target: easiest setup.

### Homelab

```text
reverse proxy
Realmforge services
Postgres
multiple realm runtimes
optional separate storage host
optional VPN ingress
```

### Advanced

Potentially:

```text
multiple control-plane replicas
external Postgres
multiple realm hosts
object storage
central logs/metrics
regional Gate endpoints
```

Do not prematurely design the simple install around Kubernetes.

## 14. Trust boundaries

At minimum distinguish:

1. Internet/untrusted client network
2. Gate/public compatibility endpoints
3. Core/admin API
4. local client agent
5. emulator runtime network
6. database network
7. backup storage
8. secrets/signing keys

Every network edge eventually needs an explicit authentication and authorization statement.

## 15. Replaceability rule

Every third-party or protocol-specific dependency must answer:

```text
What semantic service does Realmforge need from it?
What is the smallest stable contract?
What state does it own?
How is that state migrated away?
How do we black-box test a replacement?
```

If those answers do not exist, the dependency has too much architectural power.

## 16. Current unresolved architecture decisions

Do not fill these silently:

- implementation language(s) for Core/Forge/Client,
- desktop framework,
- API framework,
- event transport,
- production database topology,
- first emulator adapter,
- exact interim Gate integration strategy,
- whether Gate remains a separate service permanently,
- whether desktop-app compatibility is a launch requirement,
- public-server versus homelab-first security profiles,
- product-wide license.

Track resolution in `docs/DECISIONS.md` or ADRs.
