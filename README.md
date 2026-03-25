# ⚡ Optimus

![Version](https://img.shields.io/badge/version-1.0.0-blue?style=for-the-badge)
![License](https://img.shields.io/badge/license-MIT-green?style=for-the-badge)
![Rust](https://img.shields.io/badge/Built_with-Rust-black?style=for-the-badge&logo=rust)
![Platform](https://img.shields.io/badge/platform-windows-0078d7?style=for-the-badge&logo=windows)

> **Zero-overhead, context-aware process priority manager.** > Say goodbye to clunky 90s interfaces. Optimus delivers enterprise-grade process management with a modern, glass-morphism UI. Built on a lightning-fast Rust core and a React frontend.

Optimus allows you to intelligently control Windows process priorities. Unlike legacy tools that blindly force priorities and consume CPU cycles, Optimus uses a Smart Context Engine to manage your system resources exactly when you need them—and sleeps when you don't.

---
## 🖼 Screenshots

<p align="center">
  <img src="assets/home.png" width="400" alt="Main UI">
  <img src="assets/setting.png" width="400" alt="Settings View">
</p> 

## 🔥 Key Features

### 🧠 Core Engine (Rust-Powered)
- **Zero-Overhead Watchdog:** A background `std::thread` loop written in pure Rust. It operates entirely independently of the UI.
- **Smart Polling (Read/Compare/Set):** Optimus doesn't spam Windows API commands. It reads the current priority, compares it to your config, and executes `SetPriorityClass` *only* if there's a mismatch. 
- **Atomic File I/O:** Your configurations are bulletproof. Optimus uses temporary file writes + `sync_all` + atomic rename operations wrapped in an `RwLock`. No data corruption, even during a sudden power loss.
- **Native UAC Elevation:** Seamlessly request Administrator privileges via `ShellExecuteW` with graceful fallback and UI synchronization.

### 🎯 Smart Enforcement Modes
Choose how aggressively Optimus manages your system:
- **Mode [0] Off:** Manual application only.
- **Mode [1] Always:** Persistent 24/7 enforcement. Perfect for streaming software (OBS) or critical background tasks. 
- **Mode [2] Smart:** Context-aware enforcement. The priority is held *only* while a linked Trigger App (e.g., your favorite game) is running. When you close the game, Optimus releases the priority lock to save resources.

### ⚡ Hyper-Optimized Performance
- **IPC Delta Polling:** The React frontend caches process icons (Base64) using deterministic hashed Exe-path keys. The Rust backend sends Delta-updates (only sending icons the UI doesn't have), reducing IPC payload size by 99%.
- **O(1) Process Indexing:** The Watchdog builds a `HashMap` index of running processes every tick. Enforcement lookups happen instantly without nested O(N^2) loops.
- **React.memo Optimization:** The UI utilizes strict memoization and custom comparators. Only the specific row of a process that changed priority will rerender, eliminating UI micro-stutters.

### 🎨 Premium UI/UX
- **Dark Monochrome Aesthetic:** Clean Zinc/White color palette with glass-card components and smooth transitions.
- **Floating Navigation:** Custom frameless Tauri window with integrated Window controls and a floating settings dock.
- **Live Mode Selector:** A sleek popover to quickly switch between *Always* and *Smart* enforcement modes.
- **Grouped Process View:** Processes are automatically grouped by application with expand/collapse functionality.

### 🛡️ Security & Stability
- **Strict CSP:** Built with a rigid Content Security Policy (`default-src 'self'`) to prevent XSS and injection attacks via malicious executable names.
- **Native Crash Guard:** Startup failures are handled gracefully with native Windows dialogs (`MessageBoxW`), no silent panics.
- **Autostart & Tray:** Includes minimize-to-tray behavior and `tauri-plugin-autostart` for seamless daily use.

---

## 🛠️ Architecture

Optimus operates on a decoupled architecture:
1. **The Backend (`src-tauri/src/main.rs`):** Interacts with Windows APIs (`sysinfo`, WinAPI). Handles the 5-second Watchdog loop, file system locks, and raw process manipulation.
2. **The Bridge (Tauri IPC):** Facilitates communication using a highly optimized Delta-update protocol to keep memory usage negligible.
3. **The Frontend (`src/App.tsx`):** A React/TypeScript application styled with Tailwind CSS, responsible purely for state presentation and configuration building.

---

## 🚀 Installation & Build Instructions

### Prerequisites
- [Rust](https://www.rust-lang.org/tools/install) (latest stable)
- [Node.js](https://nodejs.org/) (v18+)
- [Tauri CLI](https://tauri.app/v1/guides/getting-started/setup/)

### Development
1. Clone the repository:
   ```bash
   git clone [https://github.com/yourusername/optimus.git](https://github.com/yourusername/optimus.git)
   cd optimus
