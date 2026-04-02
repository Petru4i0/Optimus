import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";
import { IconBinaryDto } from "../types/icon";

const iconUrlPool = new Map<string, { url: string; refs: number }>();

export function useIconObjectUrl(iconKey: string | null | undefined) {
  const [url, setUrl] = useState<string | null>(null);
  const mountedKeyRef = useRef<string | null>(null);

  const iconQuery = useQuery({
    queryKey: ["icon", iconKey],
    enabled: Boolean(iconKey),
    staleTime: Infinity,
    gcTime: 10 * 60 * 1000,
    queryFn: async () => invoke<IconBinaryDto>("icon_get_png", { iconKey }),
  });

  useEffect(() => {
    const releaseMounted = () => {
      const mountedKey = mountedKeyRef.current;
      if (!mountedKey) {
        return;
      }
      const entry = iconUrlPool.get(mountedKey);
      if (!entry) {
        mountedKeyRef.current = null;
        return;
      }
      if (entry.refs <= 1) {
        URL.revokeObjectURL(entry.url);
        iconUrlPool.delete(mountedKey);
      } else {
        iconUrlPool.set(mountedKey, {
          ...entry,
          refs: entry.refs - 1,
        });
      }
      mountedKeyRef.current = null;
    };

    if (!iconKey || !iconQuery.data) {
      releaseMounted();
      setUrl(null);
      return;
    }

    if (mountedKeyRef.current && mountedKeyRef.current !== iconKey) {
      releaseMounted();
      setUrl(null);
    }

    const existing = iconUrlPool.get(iconKey);
    if (existing) {
      if (mountedKeyRef.current !== iconKey) {
        iconUrlPool.set(iconKey, {
          ...existing,
          refs: existing.refs + 1,
        });
        mountedKeyRef.current = iconKey;
      }
      setUrl(existing.url);
      return;
    }

    const bytes = new Uint8Array(iconQuery.data.bytes);
    const objectUrl = URL.createObjectURL(
      new Blob([bytes], { type: iconQuery.data.contentType ?? "image/png" }),
    );

    iconUrlPool.set(iconKey, { url: objectUrl, refs: 1 });
    mountedKeyRef.current = iconKey;
    setUrl(objectUrl);

    return () => {
      releaseMounted();
    };
  }, [iconKey, iconQuery.data]);

  useEffect(() => {
    return () => {
      const mountedKey = mountedKeyRef.current;
      if (!mountedKey) {
        return;
      }
      const entry = iconUrlPool.get(mountedKey);
      if (!entry) {
        mountedKeyRef.current = null;
        return;
      }
      if (entry.refs <= 1) {
        URL.revokeObjectURL(entry.url);
        iconUrlPool.delete(mountedKey);
      } else {
        iconUrlPool.set(mountedKey, {
          ...entry,
          refs: entry.refs - 1,
        });
      }
      mountedKeyRef.current = null;
    };
  }, []);

  return url;
}
