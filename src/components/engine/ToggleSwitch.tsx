type ToggleSwitchProps = {
  checked: boolean;
  disabled?: boolean;
  onChange: (next: boolean) => void;
  ariaLabel: string;
};

export default function ToggleSwitch({ checked, disabled, onChange, ariaLabel }: ToggleSwitchProps) {
  return (
    <label className="relative inline-flex h-6 w-11 cursor-pointer items-center">
      <input
        type="checkbox"
        className="peer sr-only"
        checked={checked}
        disabled={disabled}
        onChange={(event) => onChange(event.target.checked)}
        aria-label={ariaLabel}
      />
      <span className="absolute inset-0 rounded-full border border-zinc-800 bg-zinc-800 transition-colors peer-checked:border-zinc-500 peer-checked:bg-zinc-700 peer-disabled:cursor-not-allowed peer-disabled:opacity-50" />
      <span className="absolute left-[2px] top-[2px] h-5 w-5 rounded-full bg-zinc-300 transition-transform peer-checked:translate-x-5 peer-checked:bg-zinc-200 peer-disabled:opacity-50" />
    </label>
  );
}

