import { afterEach, describe, expect, it, vi } from "vitest"

import { panelFetch } from "./panel-fetch"

/** Capture what `panelFetch` hands to the real fetch. */
function stubFetch() {
  const calls: Array<{ input: string; init?: RequestInit }> = []
  vi.stubGlobal(
    "fetch",
    vi.fn(async (input: string, init?: RequestInit) => {
      calls.push({ input, init })
      return new Response("{}", { status: 200 })
    }),
  )
  return calls
}

afterEach(() => {
  vi.unstubAllGlobals()
})

describe("panelFetch", () => {
  it("stamps X-IA2-Origin: hmi on the panel's mutating calls (write + alarm ack)", async () => {
    const calls = stubFetch()
    await panelFetch("/write", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ name: "pump_cmd", value: 1 }),
    })
    await panelFetch("/alarms/high_level/ack", { method: "POST" })

    expect(calls).toHaveLength(2)
    for (const call of calls) {
      const headers = new Headers(call.init?.headers)
      expect(headers.get("X-IA2-Origin")).toBe("hmi")
    }
    // Caller-supplied headers survive the merge.
    const writeHeaders = new Headers(calls[0].init?.headers)
    expect(writeHeaders.get("Content-Type")).toBe("application/json")
  })

  it("stamps reads too, mirroring the IDE's apiFetch convention", async () => {
    const calls = stubFetch()
    await panelFetch("/status")
    const headers = new Headers(calls[0].init?.headers)
    expect(headers.get("X-IA2-Origin")).toBe("hmi")
  })

  it("lets an explicit caller-set origin win", async () => {
    const calls = stubFetch()
    await panelFetch("/write", {
      method: "POST",
      headers: { "X-IA2-Origin": "kiosk-3" },
    })
    const headers = new Headers(calls[0].init?.headers)
    expect(headers.get("X-IA2-Origin")).toBe("kiosk-3")
  })
})
