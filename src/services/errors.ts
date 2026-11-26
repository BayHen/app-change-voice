/** Tauri rejects with structured { code, message } as well as Error/string values. */
export function errorMessage(reason: unknown): string {
  if (reason instanceof Error) return reason.message;
  if (typeof reason === "string") return reason;
  if (
    reason &&
    typeof reason === "object" &&
    "message" in reason &&
    typeof reason.message === "string"
  )
    return reason.message;
  return "The operation failed. Please retry and check the audio device settings.";
}
