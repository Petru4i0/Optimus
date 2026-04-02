import { AnimatePresence, motion } from "framer-motion";
import { type ReactNode, useMemo, useState } from "react";
import type { Translation } from "../locales/types";
import { OptimizationStatus } from "../types/config";
import {
  AdvancedSubFeature,
  NetSniperSubFeature,
  PowerSubFeature,
  TelemetrySubFeature,
} from "../types/engine";
import InfoTooltip from "./ui/InfoTooltip";

export type OptimizationViewProps = {
  isAdmin: boolean;
  onRequireAdmin: () => void;
  status: OptimizationStatus | null;
  loading: boolean;
  telemetryBusy: boolean;
  netSniperBusy: boolean;
  powerBusy: boolean;
  advancedBusy: boolean;
  onToggleTelemetry: (subFeature: TelemetrySubFeature, enabled: boolean) => void;
  onToggleNetSniper: (subFeature: NetSniperSubFeature, enabled: boolean) => void;
  onTogglePowerMode: (subFeature: PowerSubFeature, enabled: boolean) => void;
  onToggleAdvanced: (subFeature: AdvancedSubFeature, enabled: boolean) => void;
};

type CategoryId = "internet" | "telemetry" | "power" | "advanced";
type VerifyState = "verified" | "not_verified" | "unknown";

function resolveVerifyState(applied: boolean, readable?: boolean): VerifyState {
  if (readable === false) {
    return "unknown";
  }
  return applied ? "verified" : "not_verified";
}

function combineReadable(...readables: Array<boolean | undefined>): boolean | undefined {
  if (readables.some((value) => value === false)) {
    return false;
  }
  if (readables.every((value) => value === true)) {
    return true;
  }
  return undefined;
}

function verifyStateLabel(state: VerifyState) {
  if (state === "verified") {
    return "Verified";
  }
  if (state === "unknown") {
    return "Unverifiable";
  }
  return "Not verified";
}

type FeatureRowSchema = {
  id: string;
  tooltipKey: keyof Translation["tooltips"];
  title: string;
  description: string;
  verified: boolean;
  readable?: boolean;
  checked: boolean;
  onToggle: (next: boolean) => void;
};

type CategorySchema = {
  id: CategoryId;
  title: string;
  subtitle: string;
  verified: boolean;
  readable?: boolean;
  busy: boolean;
  masterChecked: boolean;
  onToggleMaster: (next: boolean) => void;
  rows: FeatureRowSchema[];
};

function Toggle({
  checked,
  disabled,
  onChange,
  ariaLabel,
}: {
  checked: boolean;
  disabled: boolean;
  onChange: (next: boolean) => void;
  ariaLabel: string;
}) {
  return (
    <label className="relative inline-flex h-5 w-10 items-center">
      <input
        type="checkbox"
        className="peer sr-only"
        checked={checked}
        disabled={disabled}
        onChange={(event) => onChange(event.target.checked)}
        aria-label={ariaLabel}
      />
      <span className="absolute inset-0 rounded-full border border-zinc-800 bg-zinc-800 transition-colors peer-checked:border-zinc-500 peer-checked:bg-zinc-700 peer-disabled:opacity-50" />
      <span className="absolute left-[2px] top-[2px] h-4 w-4 rounded-full bg-zinc-300 transition-transform peer-checked:translate-x-5 peer-checked:bg-zinc-200 peer-disabled:opacity-50" />
    </label>
  );
}

function VerifyDot({ state }: { state: VerifyState }) {
  return (
    <span
      className={`h-2 w-2 rounded-full ${
        state === "verified"
          ? "bg-emerald-500"
          : state === "unknown"
            ? "bg-zinc-500"
            : "bg-rose-400"
      }`}
    />
  );
}

type SubRowProps = {
  tooltipKey: keyof Translation["tooltips"];
  title: string;
  description: string;
  verified: boolean;
  readable?: boolean;
  checked: boolean;
  disabled: boolean;
  onToggle: (next: boolean) => void;
};

function SubRow({
  tooltipKey,
  title,
  description,
  verified,
  readable,
  checked,
  disabled,
  onToggle,
}: SubRowProps) {
  const verifyState = resolveVerifyState(verified, readable);
  return (
    <div className="rounded-lg border border-zinc-800 bg-zinc-900 px-3 py-2.5">
      <div className="flex items-center gap-2">
        <VerifyDot state={verifyState} />
        <div className="flex items-center gap-2">
          <p className="text-sm font-medium text-zinc-100">{title}</p>
          <InfoTooltip translationKey={tooltipKey} />
        </div>
        <span
          className={`text-[11px] ${
            verifyState === "verified"
              ? "text-emerald-500"
              : verifyState === "unknown"
                ? "text-zinc-400"
                : "text-zinc-400"
          }`}
        >
          {verifyStateLabel(verifyState)}
        </span>
        <div className="ml-auto">
          <Toggle
            checked={checked}
            disabled={disabled}
            onChange={onToggle}
            ariaLabel={`Toggle ${title}`}
          />
        </div>
      </div>
      <p className="mt-1 text-xs text-zinc-400">{description}</p>
    </div>
  );
}

type CategoryCardProps = {
  title: string;
  subtitle: string;
  open: boolean;
  busy: boolean;
  verified: boolean;
  readable?: boolean;
  masterChecked: boolean;
  onToggleOpen: () => void;
  onToggleMaster: (next: boolean) => void;
  children: ReactNode;
};

function CategoryCard({
  title,
  subtitle,
  open,
  busy,
  verified,
  readable,
  masterChecked,
  onToggleOpen,
  onToggleMaster,
  children,
}: CategoryCardProps) {
  const verifyState = resolveVerifyState(verified, readable);
  return (
    <section className="rounded-2xl border border-zinc-800 bg-zinc-900">
      <div className="flex items-center gap-3 px-4 py-3">
        <button
          className="inline-flex h-8 w-8 items-center justify-center rounded-md border border-zinc-800 bg-zinc-900 text-zinc-400 transition hover:border-zinc-500 hover:text-zinc-100"
          onClick={onToggleOpen}
          aria-label={open ? `Collapse ${title}` : `Expand ${title}`}
        >
          <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.9"
            className={`h-4 w-4 transition-transform duration-200 ${open ? "rotate-180" : ""}`}
          >
            <path d="M6 9l6 6 6-6" />
          </svg>
        </button>

        <div>
          <h3 className="text-sm font-semibold text-zinc-100">{title}</h3>
          <p className="text-xs text-zinc-400">{subtitle}</p>
        </div>

        <div className="ml-auto flex items-center gap-3">
          <span className="inline-flex items-center gap-1 text-xs text-zinc-400">
            <VerifyDot state={verifyState} />
            {verifyStateLabel(verifyState)}
          </span>
          <Toggle
            checked={masterChecked}
            disabled={busy}
            onChange={onToggleMaster}
            ariaLabel={`Toggle all in ${title}`}
          />
        </div>
      </div>

      <AnimatePresence initial={false}>
        {open ? (
          <motion.div
            key="content"
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.2, ease: "easeOut" }}
            className="overflow-hidden"
          >
            <div className="space-y-2 border-t border-zinc-800 px-4 py-3">{children}</div>
          </motion.div>
        ) : null}
      </AnimatePresence>
    </section>
  );
}

export default function OptimizationView({
  isAdmin,
  onRequireAdmin,
  status,
  loading,
  telemetryBusy,
  netSniperBusy,
  powerBusy,
  advancedBusy,
  onToggleTelemetry,
  onToggleNetSniper,
  onTogglePowerMode,
  onToggleAdvanced,
}: OptimizationViewProps) {
  const [openState, setOpenState] = useState<Record<CategoryId, boolean>>({
    internet: true,
    telemetry: true,
    power: true,
    advanced: true,
  });

  const telemetry = status?.telemetry;
  const internet = status?.netSniper;
  const power = status?.powerMode;
  const advanced = status?.advanced;

  const guardTelemetryToggle = (subFeature: TelemetrySubFeature, enabled: boolean) => {
    if (!isAdmin) {
      onRequireAdmin();
      return;
    }
    onToggleTelemetry(subFeature, enabled);
  };

  const guardNetSniperToggle = (subFeature: NetSniperSubFeature, enabled: boolean) => {
    if (!isAdmin) {
      onRequireAdmin();
      return;
    }
    onToggleNetSniper(subFeature, enabled);
  };

  const guardPowerToggle = (subFeature: PowerSubFeature, enabled: boolean) => {
    if (!isAdmin) {
      onRequireAdmin();
      return;
    }
    onTogglePowerMode(subFeature, enabled);
  };

  const guardAdvancedToggle = (subFeature: AdvancedSubFeature, enabled: boolean) => {
    if (!isAdmin) {
      onRequireAdmin();
      return;
    }
    onToggleAdvanced(subFeature, enabled);
  };

  const categories = useMemo<CategorySchema[]>(
    () => [
      {
        id: "internet",
        title: "Internet",
        subtitle: "TCP Tweaks, Registry Throttling, Cloudflare DNS",
        verified: Boolean(internet?.verified),
        readable: combineReadable(
          internet?.tcpTweaksReadable,
          internet?.registryThrottlingReadable,
          internet?.cloudflareDnsReadable,
        ),
        busy: netSniperBusy,
        masterChecked: Boolean(internet?.verified),
        onToggleMaster: (next) => guardNetSniperToggle("all", next),
        rows: [
          {
            id: "tcp_tweaks",
            tooltipKey: "tcp_tweaks",
            title: "TCP Tweaks",
            description: "Apply TcpAckFrequency=1 and TcpNoDelay=1 across interface keys.",
            verified: Boolean(internet?.tcpTweaksApplied),
            readable: internet?.tcpTweaksReadable,
            checked: Boolean(internet?.tcpTweaksApplied),
            onToggle: (next) => guardNetSniperToggle("tcp_tweaks", next),
          },
          {
            id: "registry_throttling",
            tooltipKey: "reg_throttling",
            title: "Registry Throttling",
            description: "Disable NetworkThrottlingIndex and SystemResponsiveness limits.",
            verified: Boolean(internet?.registryThrottlingApplied),
            readable: internet?.registryThrottlingReadable,
            checked: Boolean(internet?.registryThrottlingApplied),
            onToggle: (next) => guardNetSniperToggle("registry_throttling", next),
          },
          {
            id: "cloudflare_dns",
            tooltipKey: "cloudflare_dns",
            title: "Cloudflare DNS",
            description: "Set active interfaces to 1.1.1.1 and 1.0.0.1.",
            verified: Boolean(internet?.cloudflareDnsApplied),
            readable: internet?.cloudflareDnsReadable,
            checked: Boolean(internet?.cloudflareDnsApplied),
            onToggle: (next) => guardNetSniperToggle("cloudflare_dns", next),
          },
        ],
      },
      {
        id: "telemetry",
        title: "Telemetry",
        subtitle: "Services, Registry Policies, Scheduled Tasks, Hosts File",
        verified: Boolean(telemetry?.verified),
        readable: combineReadable(
          telemetry?.servicesReadable,
          telemetry?.registryPoliciesReadable,
          telemetry?.scheduledTasksReadable,
          telemetry?.hostsReadable,
        ),
        busy: telemetryBusy,
        masterChecked: Boolean(telemetry?.verified),
        onToggleMaster: (next) => guardTelemetryToggle("all", next),
        rows: [
          {
            id: "services",
            tooltipKey: "tel_services",
            title: "Services",
            description: "Stop and disable DiagTrack + dmwappushservice.",
            verified: Boolean(telemetry?.servicesDisabled),
            readable: telemetry?.servicesReadable,
            checked: Boolean(telemetry?.servicesDisabled),
            onToggle: (next) => guardTelemetryToggle("services", next),
          },
          {
            id: "registry_policies",
            tooltipKey: "tel_policies",
            title: "Registry Policies",
            description: "Set AllowTelemetry=0 in DataCollection policies.",
            verified: Boolean(telemetry?.registryPoliciesDisabled),
            readable: telemetry?.registryPoliciesReadable,
            checked: Boolean(telemetry?.registryPoliciesDisabled),
            onToggle: (next) => guardTelemetryToggle("registry_policies", next),
          },
          {
            id: "scheduled_tasks",
            tooltipKey: "tel_tasks",
            title: "Scheduled Tasks",
            description: "Disable CEIP and Application Experience telemetry tasks.",
            verified: Boolean(telemetry?.scheduledTasksDisabled),
            readable: telemetry?.scheduledTasksReadable,
            checked: Boolean(telemetry?.scheduledTasksDisabled),
            onToggle: (next) => guardTelemetryToggle("scheduled_tasks", next),
          },
          {
            id: "hosts_block",
            tooltipKey: "tel_hosts",
            title: "Hosts File",
            description: "Block known telemetry endpoints via hosts entries.",
            verified: Boolean(telemetry?.hostsBlocked),
            readable: telemetry?.hostsReadable,
            checked: Boolean(telemetry?.hostsBlocked),
            onToggle: (next) => guardTelemetryToggle("hosts_block", next),
          },
        ],
      },
      {
        id: "power",
        title: "Power",
        subtitle: "Ultimate Plan and Core Parking",
        verified: Boolean(power?.verified),
        readable: combineReadable(power?.ultimatePlanReadable, power?.coreParkingReadable),
        busy: powerBusy,
        masterChecked: Boolean(power?.verified),
        onToggleMaster: (next) => guardPowerToggle("all", next),
        rows: [
          {
            id: "ultimate_plan",
            tooltipKey: "ultimate_plan",
            title: "Ultimate Plan",
            description: "Activate the Ultimate Performance power scheme.",
            verified: Boolean(power?.ultimatePlanActive),
            readable: power?.ultimatePlanReadable,
            checked: Boolean(power?.ultimatePlanActive),
            onToggle: (next) => guardPowerToggle("ultimate_plan", next),
          },
          {
            id: "core_parking",
            tooltipKey: "core_parking",
            title: "Core Parking",
            description: "Disable core parking for the current scheme.",
            verified: Boolean(power?.coreParkingDisabled),
            readable: power?.coreParkingReadable,
            checked: Boolean(power?.coreParkingDisabled),
            onToggle: (next) => guardPowerToggle("core_parking", next),
          },
        ],
      },
      {
        id: "advanced",
        title: "Advanced",
        subtitle: "HPET/Timer Overrides, Network IRQ Moderation, MMCSS",
        verified: Boolean(advanced?.verified),
        readable: combineReadable(
          advanced?.hpetDynamicTickReadable,
          advanced?.interruptModerationReadable,
          advanced?.mmcssReadable,
        ),
        busy: advancedBusy,
        masterChecked: Boolean(advanced?.verified),
        onToggleMaster: (next) => guardAdvancedToggle("all", next),
        rows: [
          {
            id: "hpet_dynamic_tick",
            tooltipKey: "hpet_dynamic_tick",
            title: "HPET & Dynamic Tick",
            description: "Remove HPET overrides and force disabledynamictick=yes.",
            verified: Boolean(advanced?.hpetDynamicTickApplied),
            readable: advanced?.hpetDynamicTickReadable,
            checked: Boolean(advanced?.hpetDynamicTickApplied),
            onToggle: (next) => guardAdvancedToggle("hpet_dynamic_tick", next),
          },
          {
            id: "interrupt_moderation",
            tooltipKey: "interrupt_moderation",
            title: "Interrupt Moderation",
            description: "Disable NIC interrupt batching on active Ethernet and Wi-Fi adapters.",
            verified: Boolean(advanced?.interruptModerationApplied),
            readable: advanced?.interruptModerationReadable,
            checked: Boolean(advanced?.interruptModerationApplied),
            onToggle: (next) => guardAdvancedToggle("interrupt_moderation", next),
          },
          {
            id: "mmcss",
            tooltipKey: "mmcss",
            title: "MMCSS Injection",
            description: "Prioritize the Games multimedia profile and set SystemResponsiveness to 0.",
            verified: Boolean(advanced?.mmcssApplied),
            readable: advanced?.mmcssReadable,
            checked: Boolean(advanced?.mmcssApplied),
            onToggle: (next) => guardAdvancedToggle("mmcss", next),
          },
        ],
      },
    ],
    [
      advanced?.hpetDynamicTickApplied,
      advanced?.hpetDynamicTickReadable,
      advanced?.interruptModerationApplied,
      advanced?.interruptModerationReadable,
      advanced?.mmcssApplied,
      advanced?.mmcssReadable,
      advanced?.verified,
      advancedBusy,
      guardAdvancedToggle,
      internet?.cloudflareDnsApplied,
      internet?.cloudflareDnsReadable,
      internet?.registryThrottlingApplied,
      internet?.registryThrottlingReadable,
      internet?.tcpTweaksApplied,
      internet?.tcpTweaksReadable,
      internet?.verified,
      netSniperBusy,
      guardNetSniperToggle,
      onToggleAdvanced,
      guardPowerToggle,
      guardTelemetryToggle,
      power?.coreParkingDisabled,
      power?.coreParkingReadable,
      power?.ultimatePlanActive,
      power?.ultimatePlanReadable,
      power?.verified,
      powerBusy,
      telemetry?.hostsBlocked,
      telemetry?.hostsReadable,
      telemetry?.registryPoliciesDisabled,
      telemetry?.registryPoliciesReadable,
      telemetry?.scheduledTasksDisabled,
      telemetry?.scheduledTasksReadable,
      telemetry?.servicesDisabled,
      telemetry?.servicesReadable,
      telemetry?.verified,
      telemetryBusy,
    ],
  );

  return (
    <section className="glass-card rounded-2xl p-5">
      <div className="flex items-center gap-3">
        <div>
          <h2 className="text-lg font-semibold text-zinc-100">Optimization</h2>
          <p className="mt-1 text-xs text-zinc-400">
            Granular controls with per-feature verification.
          </p>
        </div>
        <div className="ml-auto text-xs text-zinc-400">
          {loading ? "Syncing..." : "Diagnostics synced"}
        </div>
      </div>

      <div className="mt-4 space-y-3">
        {categories.map((category) => (
          <CategoryCard
            key={category.id}
            title={category.title}
            subtitle={category.subtitle}
            open={openState[category.id]}
            busy={category.busy}
            verified={category.verified}
            readable={category.readable}
            masterChecked={category.masterChecked}
            onToggleOpen={() =>
              setOpenState((prev) => ({
                ...prev,
                [category.id]: !prev[category.id],
              }))
            }
            onToggleMaster={category.onToggleMaster}
          >
            {category.rows.map((row) => (
              <SubRow
                key={row.id}
                tooltipKey={row.tooltipKey}
                title={row.title}
                description={row.description}
                verified={row.verified}
                readable={row.readable}
                checked={row.checked}
                disabled={!row.readable || category.busy}
                onToggle={row.onToggle}
              />
            ))}
          </CategoryCard>
        ))}
      </div>
    </section>
  );
}
