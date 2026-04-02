import type { Translation } from "./types";

export const en: Translation = {
  welcome: {
    title: "Hiiii! I'm Optimus! 🐾✨",
    text: "I'm your tiny but super hardworking digital helper! 🐹 My absolute biggest dream is to make your computer the fastest and happiest in the world! 🚀💕 But... 🥺 I have one little fear. Big, grumpy uncle-Antiviruses 👹 just don't understand my magic. They think that anyone cleaning the system this deeply must be a bad guy... They might yell at me, show scary red warnings, or even try to lock me in quarantine! 😭 Please protect me! 🙏 Add me to your whitelist, and I will faithfully serve you every single day! I really, really want to be friends! 🥺💖",
    buttonWait: "I'll protect you! 🛡️ ({s}s)",
    buttonReady: "I'll protect you! 🛡️",
  },
  tooltips: {
    turbo_timer:
      "Forces Windows timer resolution to 0.5 ms for tighter scheduler wake timing and more consistent input latency.",
    memory_purge:
      "Runs standby list cleanup (cached RAM) to release memory held by inactive processes and reduce stutter under load.",
    live_mode:
      "Smart: Applies optimizations only when the game is in focus. Always: Enforces optimizations as long as the process is running.",
    watchdog:
      "Runs a lightweight 60-second background service loop that re-checks and restores optimization state drift.",
    auto_purge_triggers:
      "Automatically clears the Standby List when Free Memory drops below the selected threshold, preventing mid-game stutters.",
    msi_utility:
      "Switches devices from legacy Line-Based Interrupts to Message Signaled Interrupts (MSI). Drastically reduces DPC latency and CPU overhead for GPUs and USB controllers.",
    msi_priority:
      "Forces Windows to process hardware interrupts for this specific device first. Set GPU/Network to High, leave others Default.",
    driver_store:
      "Scans the protected Windows FileRepository for old, duplicated, or orphaned driver packages that bloat your C: drive.",
    force_delete:
      "Bypasses standard API locks to aggressively remove the driver or registry key. Use with extreme caution.",
    ghost_devices:
      "Lists disconnected devices that still retain stale registry entries and assigned resources.",
    tcp_tweaks:
      "Optimizes TCP stack by disabling Nagle's Algorithm and tuning AckFrequency. Reduces network latency and jitter in online games.",
    reg_throttling:
      "Removes the network throttling index and sets system responsiveness to 0, ensuring games get 100% of CPU/Network priority.",
    cloudflare_dns:
      "Configures active interfaces to use Cloudflare's 1.1.1.1 (IPv4) and IPv6 equivalents for faster, private DNS resolution.",
    tel_services:
      "Stops and disables DiagTrack and dmwappushservice, freeing up background CPU cycles and RAM.",
    tel_policies:
      "Applies hard registry policies to block Windows data collection, even if services attempt to restart.",
    tel_tasks:
      "Disables scheduled telemetry tasks (CEIP, App Experience) that wake up your PC to scan and upload reports.",
    tel_hosts:
      "Redirects known Microsoft telemetry domains to 0.0.0.0 via the hosts file for network-level blocking.",
    ultimate_plan:
      "Unlocks the hidden 'Ultimate Performance' scheme which forces the CPU to stay at maximum boost clock constantly.",
    core_parking:
      "Prevents Windows from parking CPU cores in sleep states, eliminating the latency spike when a core needs to wake up for a task.",
    hpet_dynamic_tick:
      "Removes forced HPET timer overrides and disables Dynamic Tick to keep the scheduler on a low-latency, always-awake timing path.",
    interrupt_moderation:
      "Disables NIC interrupt batching on active hardware adapters so packets are processed immediately instead of waiting in a moderation queue.",
    mmcss:
      "Raises the Games multimedia task profile and sets SystemResponsiveness to 0 so Windows favors gaming threads and GPU scheduling.",
    purgeTooltip:
      "Cleans Temp, Prefetch, Recycle Bin, Dumps, Driver leftovers, CBS logs, Launchers, Browser media, Spotify, VSCode, Telegram, and Dev caches (npm/pip/Cargo/Gradle/NuGet/Go).",
  },
};