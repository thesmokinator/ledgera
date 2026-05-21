import { invoke } from "@tauri-apps/api/core";

/** Invokes a typed Tauri command. */
export function callCommand<
  TResponse,
  TPayload extends Record<string, unknown> = Record<string, never>,
>(command: string, payload?: TPayload): Promise<TResponse> {
  return invoke<TResponse>(command, payload);
}
