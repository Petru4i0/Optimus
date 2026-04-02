export interface AppErrorEnvelope {
  code: string;
  message: string;
  requiresAdmin: boolean;
  retryable: boolean;
}

export function parseIpcError(error: unknown): AppErrorEnvelope {
  if (typeof error === "object" && error !== null) {
    const candidate = error as Partial<AppErrorEnvelope>;
    if (typeof candidate.message === "string" && typeof candidate.code === "string") {
      const normalizedCode = candidate.code.toUpperCase();
      return {
        code: candidate.code,
        message: candidate.message,
        requiresAdmin:
          typeof candidate.requiresAdmin === "boolean"
            ? candidate.requiresAdmin
            : normalizedCode === "ACCESS_DENIED",
        retryable: Boolean(candidate.retryable),
      };
    }
  }

  const message = error instanceof Error ? error.message : String(error);
  return {
    code: "INTERNAL",
    message,
    requiresAdmin: false,
    retryable: true,
  };
}
