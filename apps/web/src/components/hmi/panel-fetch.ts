/**
 * `fetch` for the standalone operator panel: every request declares
 * `X-IA2-Origin: hmi`, so panel actions are attributed to the operator
 * panel rather than an anonymous script (ADR-0002). Variable writes
 * land in the edge audit ring as `hmi`; alarm acks send the same
 * header for parity, but the edge ack handler does not record acks
 * today (the ring covers writes/forces). Mirrors the IDE's `apiFetch`,
 * which declares `gui`.
 *
 * All panel HTTP goes through here so the header can't be forgotten
 * per-call — "the operator panel sends `hmi`" (docs/api.md) must stay
 * true for every mutating call the panel ever grows, not just /write.
 */
export async function panelFetch(
  input: string,
  init?: RequestInit,
): Promise<Response> {
  const headers = new Headers(init?.headers)
  if (!headers.has("X-IA2-Origin")) {
    headers.set("X-IA2-Origin", "hmi")
  }
  return fetch(input, { ...init, headers })
}
