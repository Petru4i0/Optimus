import { useCallback, useRef } from "react";
import { ProcessGroupDto } from "../types/process";

type UseIconCacheOptions = {
  ttlMs: number;
  maxEntries: number;
};

export function useIconCache({ ttlMs, maxEntries }: UseIconCacheOptions) {
  const iconCacheRef = useRef<Map<string, string>>(new Map());
  const iconSeenAtRef = useRef<Map<string, number>>(new Map());

  const prune = useCallback(
    (now: number) => {
      for (const [key, seenAt] of iconSeenAtRef.current) {
        if (now - seenAt > ttlMs) {
          iconSeenAtRef.current.delete(key);
          iconCacheRef.current.delete(key);
        }
      }

      if (iconSeenAtRef.current.size > maxEntries) {
        const oldest = Array.from(iconSeenAtRef.current.entries())
          .sort((a, b) => a[1] - b[1])
          .slice(0, iconSeenAtRef.current.size - maxEntries);

        for (const [key] of oldest) {
          iconSeenAtRef.current.delete(key);
          iconCacheRef.current.delete(key);
        }
      }
    },
    [maxEntries, ttlMs],
  );

  const retain = useCallback((activeKeys: Set<string>) => {
    for (const key of Array.from(iconSeenAtRef.current.keys())) {
      if (activeKeys.has(key)) {
        continue;
      }
      iconSeenAtRef.current.delete(key);
      iconCacheRef.current.delete(key);
    }
  }, []);

  const hydrateGroups = useCallback((groups: ProcessGroupDto[], now: number) => {
    return groups.map((group) => {
      iconSeenAtRef.current.set(group.iconKey, now);

      if (group.iconBase64) {
        iconCacheRef.current.set(group.iconKey, group.iconBase64);
        return group;
      }

      const cached = iconCacheRef.current.get(group.iconKey);
      if (!cached) {
        return group;
      }

      return {
        ...group,
        iconBase64: cached,
      };
    });
  }, []);

  return {
    prune,
    retain,
    hydrateGroups,
  };
}
