<div align="center">
  <h1>Optimus</h1>
  <img src="https://img.shields.io/github/v/release/Petru4i0/optimus?style=for-the-badge&color=0ea5e9" alt="Version"/>
  <img src="https://img.shields.io/badge/platform-windows-0078d7?style=for-the-badge&logo=windows" alt="Windows"/>
  <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust" alt="Rust"/>
  <img src="https://img.shields.io/badge/React-20232A?style=for-the-badge&logo=react&logoColor=61DAFB" alt="React"/>
  <img src="https://img.shields.io/badge/Tauri-v2-24C8D8?style=for-the-badge&logo=tauri&logoColor=white" alt="Tauri v2"/>
  <img src="https://img.shields.io/badge/License-MIT-22c55e?style=for-the-badge" alt="License"/>
</div>

## Optimus v0.2.0
Optimus is a Windows performance control platform engineered for one target: **zero-overhead execution with brutal determinism**. It is not a cosmetic booster. It is a systems utility that manipulates process scheduling, network behavior, hardware interrupt policy, telemetry surfaces, and deep cache state using explicit, reversible operations.

## Executive Summary
The core idea is simple: do less work, but do it with precision.

- Process transport is **delta-based**, not full-snapshot spam.
- High-risk operations are **stateful and reversible** through snapshot capture.
- Readability and verification are separated into explicit status semantics: **readable**, **applied**, **verified**.
- Heavy system calls are pushed into background workers so the control plane stays responsive.

This is the opposite of blind tweak packs. Every module is structured around measurable state and controlled mutation.

## The Core Architecture
### Rust control core
The backend is a native Rust execution layer that handles:

- **WinAPI calls** for process priority, timer resolution, memory list purge, service control, security descriptor management, and shell maintenance APIs.
- **Registry orchestration** for HKLM/HKCU feature toggles and restore paths.
- **CLI orchestration** for operating-system tools like **powercfg**, **bcdedit**, **netsh**, **schtasks**, and **pnputil**.
- **Snapshot and rollback** state for optimization domains.

### React/Tauri front-end
The front-end is a React + TypeScript command surface:

- Uses modular hooks and an aggregator pattern to centralize command dispatch.
- Uses persisted Zustand state for durable workspace behavior.
- Keeps high-cost queries controlled (manual/focus-triggered) while lightweight process deltas remain periodic.

### IPC strategy
Tauri IPC is used as a typed command boundary. Large or blocking work runs in blocking workers, then returns compact DTOs to the UI. The UI does not infer machine state from assumptions; it renders backend truth.

### Build profile
Release builds are tuned for footprint and startup performance:

- **opt-level = "z"**
- **lto = true**
- **codegen-units = 1**
- **panic = "abort"**
- **strip = true**

## Module Deep-Dive
## Process Sniper
Process Sniper enforces priority policy through a low-latency diff pipeline.

### Delta process model
A sampler loop builds in-memory snapshots of running processes. The backend computes a delta against prior state and sends only:

- added rows
- changed rows
- removed PIDs

This prevents full payload churn and keeps the bridge efficient even on busy systems.

### Priority mechanics
Per-PID operations use:

- **OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION / PROCESS_SET_INFORMATION)**
- **GetPriorityClass**
- **SetPriorityClass**

Access-denied paths are surfaced as data, not crashes, so protected processes do not collapse the view.

### Icon deduplication with blake3
Icon identity is derived from executable path plus metadata. A compact **blake3** key is generated and collision-guarded. The front-end requests only missing icon binaries, which massively reduces IPC payload duplication.

## The Purifier (Deep Purge)
The Purifier is a category-driven cleanup engine with strict accounting and soft-fail semantics.

### Category controls
Deep purge executes only selected categories:

- Windows system targets
- GPU caches/install remnants
- Browser media cache targets
- Launcher and application caches
- Developer ecosystem caches

If all categories are disabled, purge exits with zero work.

### Byte-accurate accounting
Freed bytes are accumulated only when deletion actually succeeds. For recycle-bin cleanup, bytes are derived from pre/post bin size query delta.

### Safety and skip model
The cleaner treats missing/in-use/denied artifacts as skippable and continues:

- **NotFound**, **AccessDenied**, file locks, busy handles
- No fatal abort because one subtree is locked
- Prefetch root is handled with root-and-recreate fallback logic

The result is stable long-run behavior on real user systems with active processes.

## Hardware Lab
Hardware Lab exposes low-level device controls with explicit safety policy.

### MSI Utility
The MSI tool reads PCI device state and writes interrupt mode policy through device registry keys.

Key mechanics:

- Enumerates **HKLM\\SYSTEM\\CurrentControlSet\\Enum\\PCI\\...** device instances.
- Writes **MSISupported** under device interrupt properties.
- Writes device interrupt priority in affinity policy keys.

**Why this matters:** line-based interrupts are shared and can increase DPC contention. Message-signaled interrupts reduce shared-line contention by delivering interrupts as in-band messages, which can improve interrupt dispatch characteristics on supported devices.

### Driver Store cleaner
Driver inventory is assembled via DISM-oriented PowerShell path with fallback inventory path. Deletion uses controlled **pnputil** orchestration.

Deletion path characteristics:

- `pnputil /delete-driver <published> /uninstall` with optional force behavior
- timeout-bound operations for destructive commands
- guardrails against protected/critical services and classes

### Inactive devices (ghost removal)
Disconnected device enumeration is performed with a primary query and fallback query for wider compatibility. Removal is executed through **pnputil /remove-device** with retry behavior and best-effort stale-key cleanup.

## Optimization Suite
Optimization is split into four domains with independent status/readability.

## Telemetry (four layers)
Telemetry hardening is not a single switch. It is layered:

1. **Services:** disables telemetry service startup and stops service processes.
2. **Policies:** sets **AllowTelemetry** policy state in DataCollection path.
3. **Scheduled tasks:** disables CEIP/Application Experience related tasks.
4. **Hosts blocking:** injects controlled hosts block section with atomic write semantics.

Security descriptor snapshot/restore logic exists for policy key takeover and rollback edge cases.

## Network tuning
### TCP latency keys
The engine applies per-interface registry values:

- **TcpAckFrequency = 1**
- **TcpNoDelay = 1**

What they do:

- **TcpNoDelay** disables Nagle-style coalescing so small outbound packets are not delayed for aggregation.
- **TcpAckFrequency = 1** reduces delayed ACK batching behavior, causing ACKs to be sent more immediately.

Combined effect: reduced packet batching latency in interactive network flows, with tradeoff of increased packet/ACK frequency.

### Throttling controls
The suite also adjusts multimedia/network throttling behavior by setting:

- **NetworkThrottlingIndex = 0xFFFFFFFF**
- **SystemResponsiveness = 0**

### DNS policy
Active interfaces can be switched to Cloudflare DNS via netsh orchestration and verified per interface.

## Power management
Power controls use **powercfg** command paths:

- Ultimate Performance scheme duplication/activation (base GUID anchored workflow)
- Core parking disable path via processor subgroup and parking setting GUID mutation

Rollback can restore balanced defaults where snapshots are unavailable.

## Advanced controls
### HPET and Dynamic Tick
Timer override logic manipulates boot configuration through:

- `bcdedit /deletevalue useplatformclock`
- `bcdedit /set disabledynamictick yes`

Status is read from `bcdedit /enum {current}` parse, not brittle single-key get calls.

### MMCSS profile injection
MMCSS tuning writes game profile and system responsiveness values under multimedia SystemProfile and Games task profile for low-latency scheduling bias.

### Interrupt Moderation
Active adapter keys are matched and `*InterruptModeration` is normalized to disabled value, preserving underlying value type compatibility where required.

## Security and UX Flow
## Always Run as Administrator
The persistent "always elevate on launch" behavior is implemented through AppCompat layers:

- Registry path: **HKCU\\Software\\Microsoft\\Windows NT\\CurrentVersion\\AppCompatFlags\\Layers**
- Value name: absolute path to Optimus executable
- Value data: **~ RUNASADMIN**

Disabling removes that value; missing-value delete is treated as benign.

## Kind Spirit onboarding psychology
Startup flow is intentionally staged:

1. Safety intro acknowledgement
2. Onboarding for elevated startup recommendation
3. Main control surface

This reduces trust shock when UAC/AV heuristics react to low-level operations.

## UI persistence
Workspace context is persisted using Zustand persist, including major navigation and tool filter state, while intentionally leaving ephemeral form/search states non-persistent where appropriate.

## Technical Specs
### Core Win32/NT APIs in active use
- **OpenProcess**
- **GetPriorityClass**
- **SetPriorityClass**
- **NtSetTimerResolution**
- **NtQueryTimerResolution**
- **NtSetSystemInformation** (memory list purge path)
- **ChangeServiceConfigW / ControlService / StartServiceW**
- **SetNamedSecurityInfoW / GetNamedSecurityInfoW**
- **SHQueryRecycleBinW / SHEmptyRecycleBinW**

### Primary registry surfaces
- **HKLM\\SYSTEM\\CurrentControlSet\\Services\\Tcpip\\Parameters\\Interfaces\\{GUID}**
- **HKLM\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Multimedia\\SystemProfile**
- **HKLM\\SOFTWARE\\Policies\\Microsoft\\Windows\\DataCollection**
- **HKLM\\SYSTEM\\CurrentControlSet\\Enum\\PCI\\...**
- **HKCU\\Software\\Microsoft\\Windows NT\\CurrentVersion\\AppCompatFlags\\Layers**

### CLI toolchain
- **powercfg**
- **bcdedit**
- **netsh**
- **schtasks**
- **pnputil**

## Installation
### Prebuilt
1. Download latest `.exe` or `.msi` from [GitHub Releases](https://github.com/Petru4i0/optimus/releases).
2. Run installer.
3. Launch Optimus.

### Source build
```bash
git clone https://github.com/Petru4i0/optimus.git
cd optimus
npm install
npm run tauri dev
```

Release build:
```bash
npm run tauri build
```

## Usage Flow
1. Pass Safety Intro.
2. Complete onboarding choice for elevated startup.
3. Configure process maps and trigger rules.
4. Use Engine, Hardware Lab, and Optimization domains as needed.
5. Keep Optimus running in tray/background when desired.

## Operational Boundaries
- Administrator privileges are required for many HKLM, service, boot config, and device operations.
- Hardware and optimization operations can alter system behavior; snapshots and verification should be used deliberately.
- Deep purge is cache/log oriented and intentionally skips locked or unavailable artifacts.

## Contributing
1. Fork repository.
2. Create a focused feature branch.
3. Validate behavior with build/check gates.
4. Open PR with technical rationale and test evidence.

## Credits
- **Creator & Lead Developer:** PetruchiO
- **License:** MIT (`LICENSE`)
