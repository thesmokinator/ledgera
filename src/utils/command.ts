import { invoke } from "@tauri-apps/api/core";

/** Invokes a typed Tauri command. */
export function callCommand<TResponse>(
  command: string,
  payload?: Record<string, unknown>,
): Promise<TResponse> {
  return invoke<TResponse>(command, payload);
}
