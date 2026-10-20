export interface PrefixResolution {
  effectiveQuery: string;
  hasPrefix: boolean;
  prefix: string | null;
}

/**
 * Resolves Flow Launcher-style search prefixes.
 * Strips prefix only when it appears as a standalone leading token followed by whitespace.
 */
export function resolvePrefix(
  rawQuery: string,
  enabled: boolean,
  prefixes: readonly string[],
): PrefixResolution {
  if (!enabled || !rawQuery) {
    return { effectiveQuery: rawQuery, hasPrefix: false, prefix: null };
  }

  const trimmedStart = rawQuery.trimStart();
  if (!trimmedStart) {
    return { effectiveQuery: rawQuery, hasPrefix: false, prefix: null };
  }

  for (const prefix of prefixes) {
    const cleanPrefix = prefix.trim();
    if (!cleanPrefix) continue;

    if (trimmedStart.toLowerCase().startsWith(cleanPrefix.toLowerCase())) {
      const rest = trimmedStart.slice(cleanPrefix.length);
      // Must be followed by whitespace or be an exact standalone token
      if (rest.length === 0) {
        return {
          effectiveQuery: "",
          hasPrefix: true,
          prefix: cleanPrefix,
        };
      }
      if (/^\s/.test(rest)) {
        return {
          effectiveQuery: rest.trimStart(),
          hasPrefix: true,
          prefix: cleanPrefix,
        };
      }
    }
  }

  return { effectiveQuery: rawQuery, hasPrefix: false, prefix: null };
}
