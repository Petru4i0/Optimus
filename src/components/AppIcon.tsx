import { useEffect, useState } from "react";
import { useIconObjectUrl } from "../hooks/useIconObjectUrl";

type AppIconProps = {
  appName: string;
  iconBase64?: string | null;
  iconKey?: string | null;
  className?: string;
};

export default function AppIcon({
  appName,
  iconBase64,
  iconKey,
  className = "h-8 w-8",
}: AppIconProps) {
  const iconObjectUrl = useIconObjectUrl(iconKey);
  const iconSrc = iconBase64 ?? iconObjectUrl;
  const [loadFailed, setLoadFailed] = useState(false);

  useEffect(() => {
    setLoadFailed(false);
  }, [iconSrc]);

  if (iconSrc && !loadFailed) {
    return (
      <img
        src={iconSrc}
        alt={appName}
        className={`${className} object-contain`}
        onError={() => setLoadFailed(true)}
      />
    );
  }

  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.7"
      className={`${className} text-zinc-400/85`}
      aria-hidden="true"
    >
      <rect x="4" y="4" width="16" height="16" rx="3.5" />
      <path d="M9 9h6v6H9z" />
      <path d="M3 12h2" />
      <path d="M19 12h2" />
      <path d="M12 3v2" />
      <path d="M12 19v2" />
    </svg>
  );
}
