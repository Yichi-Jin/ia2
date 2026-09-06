import { describe, expect, it } from "vitest"

import { activityLabel } from "./agent-activity"

const base = { command: null, sessionLabel: null, recent: [] as Array<{ command: string; ts: number }> }

describe("activityLabel", () => {
  it("renders the server's label verbatim — never invents a `cs ` prefix", () => {
    // Auto-attributed external writers arrive with the origin already
    // in the label; prefixing `cs` here would claim the CLI wrote
    // something it never did (the pre-fix banner bug).
    const label = activityLabel({
      ...base,
      command: "write x — mqtt (self-declared)",
    })
    expect(label).toBe("write x — mqtt (self-declared)")
    expect(label.startsWith("cs ")).toBe(false)

    // A real cs heartbeat's prefix comes from the SERVER, not from us.
    expect(activityLabel({ ...base, command: "cs runtime write" })).toBe(
      "cs runtime write",
    )
    expect(
      activityLabel({ ...base, command: "write x (unattributed)" }),
    ).toBe("write x (unattributed)")
  })

  it("shows the session label when no flash is pending", () => {
    expect(
      activityLabel({ ...base, sessionLabel: "rebuilding tank controller" }),
    ).toBe("rebuilding tank controller")
  })

  it("lets an external writer's flash override the session label", () => {
    // The contract: a non-cs writer surfaces even during an active
    // session; the server later reverts command to null and the
    // banner falls back to the session label.
    expect(
      activityLabel({
        ...base,
        command: "write x — hmi (self-declared)",
        sessionLabel: "rebuilding tank controller",
      }),
    ).toBe("write x — hmi (self-declared)")
  })

  it("falls back to recent history, then a placeholder", () => {
    expect(
      activityLabel({ ...base, recent: [{ command: "cs set pous/main", ts: 1 }] }),
    ).toBe("cs set pous/main")
    expect(activityLabel(base)).toBe("working")
  })
})
