#!/usr/bin/env python3
"""Recompute every headline number in docs/bench/*.md from the raw CSVs
in this directory. Stdlib only (csv, statistics) — no dependency on any
IA2 code, so the check stays independent of the thing it verifies.

Run:  python3 docs/bench/data/recompute.py
Exit: 0 = every published number reproduces within its stated tolerance;
      1 = at least one number failed to reproduce (printed).
"""

import csv
import statistics
import sys
from pathlib import Path

HERE = Path(__file__).parent
FAILURES: list[str] = []

ENCODER_CNT_PER_REV = 8_388_608  # SV660N / MS1H4 23-bit absolute encoder


def check(label: str, got: float, want: float, tol: float) -> None:
    ok = abs(got - want) <= tol
    print(f"  {'ok  ' if ok else 'FAIL'} {label}: got {got:.4f}, published {want:.4f} (tol {tol})")
    if not ok:
        FAILURES.append(label)


def rows(name: str) -> list[dict]:
    with open(HERE / name, newline="") as fh:
        return list(csv.DictReader(fh))


# ---------------------------------------------------------------- scan cadence
def scan_cadence() -> None:
    print("scan-cadence-ethercat-only-20260728.csv (2 ms task, EtherCAT only)")
    rates = [float(r["scan_rate"]) for r in rows("scan-cadence-ethercat-only-20260728.csv") if r["scan_rate"]]
    check("mean scan rate [/s]", statistics.mean(rates), 500.0, 0.5)
    check("min scan rate [/s]", min(rates), 495.3, 0.5)
    dur = len(rates)  # one sample every ~0.2 s → duration sanity only
    print(f"       samples: {dur} (≈{dur * 0.2:.0f} s of logging)")

    print("scan-cadence-mixed-bus-20260709.csv (2 ms task, EtherCAT + Modbus RTU coupler, pre-fix)")
    mixed = rows("scan-cadence-mixed-bus-20260709.csv")
    rates = [float(r["scan_rate"]) for r in mixed if r["scan_rate"]]
    check("mean scan rate [/s]", statistics.mean(rates), 397.1, 1.0)
    check("min scan rate [/s]", min(rates), 357.9, 1.0)
    bad = sum(1 for r in mixed if r["coupler_ok"] != "1")
    check("coupler_ok != 1 samples", bad, 0, 0)
    print(f"       coupler_ok = 1 on {len(mixed) - bad}/{len(mixed)} samples")


def scan_cadence_postfix() -> None:
    print("scan-cadence-mixed-bus-postfix-20260908.csv (2 ms task, EtherCAT + RTU coupler, POST-fix)")
    data = rows("scan-cadence-mixed-bus-postfix-20260908.csv")
    rates = []
    for a, b in zip(data, data[1:]):
        dt = (int(b["ts_us"]) - int(a["ts_us"])) / 1e6
        if dt > 0.05:
            rates.append((int(b["scan_count"]) - int(a["scan_count"])) / dt)
    check("mean scan rate [/s]", statistics.mean(rates), 500.0, 0.5)
    check("min scan rate [/s]", min(rates), 493.6, 0.5)
    unhealthy = sum(1 for r in data if r["devices_healthy"] != "1")
    check("unhealthy samples", unhealthy, 0, 0)

    print("long-uptime-counters-20260908.csv (continuous-operation counter snapshot)")
    lu = rows("long-uptime-counters-20260908.csv")
    last = lu[-1]
    days = int(last["uptime_secs"]) / 86400
    avg = int(last["scan_count"]) / int(last["uptime_secs"])
    check("lifetime mean scan rate [/s]", avg, 499.7, 0.1)
    check("continuous uptime [days]", days, 7.84, 0.05)


def cable_pull() -> None:
    print("cable-pull-journal-20260908.log (bus-loss self-heal, journal excerpt)")
    import re
    t_changed = t_recovered = None
    pat = re.compile(r"(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+)Z")
    for line in open(HERE / "cable-pull-journal-20260908.log"):
        if "bus shape CHANGED" in line and t_changed is None:
            t_changed = pat.search(line).group(1)
        if "recovered; cyclic exchange running again" in line:
            t_recovered = pat.search(line).group(1)
    from datetime import datetime
    dt = (
        datetime.fromisoformat(t_recovered) - datetime.fromisoformat(t_changed)
    ).total_seconds()
    check("replug-to-OP [s]", dt, 2.08, 0.05)
    text = open(HERE / "cable-pull-journal-20260908.log").read()
    check("re-walk backoff engaged", int("re-walk backoff attempt=1" in text), 1, 0)
    check("transport rebuild path exercised", int("supervise loop will rebuild the transport" in text), 1, 0)


# ---------------------------------------------------------------- dual gear
def phase_ratio(data: list[dict], phase: str) -> float:
    """Steady-state actual/actual ratio, mid-segment (1/4..3/4 of the
    phase) to skip the soft-engagement ramps — the same window the
    original bench script used, so the published digits reproduce."""
    seg = [r for r in data if r["phase"] == phase]
    i, j = len(seg) // 4, len(seg) * 3 // 4
    d0 = float(seg[j]["act0"]) - float(seg[i]["act0"])
    d1 = float(seg[j]["act1"]) - float(seg[i]["act1"])
    return d0 / d1


def dual_gear() -> None:
    print("dual-gear-20260708.csv (two SV660N axes, 2 ms / CSP / SYNC0)")
    data = rows("dual-gear-20260708.csv")
    check("gear ratio 1:1 forward", phase_ratio(data, "gear-1to1-fwd"), 1.0026, 0.0005)
    check("gear ratio 2:1 forward", phase_ratio(data, "gear-2to1-fwd"), 1.9979, 0.0005)
    check("gear ratio 1:1 reverse", phase_ratio(data, "gear-1to1-rev"), 0.9998, 0.0005)
    check("gear ratio 2:1 reverse", phase_ratio(data, "gear-2to1-rev"), 2.0000, 0.0005)

    def ferr_deg(phase: str):
        seg = [abs(float(r["ferr0"])) for r in data if r["phase"] == phase]
        to_deg = 360.0 / ENCODER_CNT_PER_REV
        return statistics.mean(seg) * to_deg, max(seg) * to_deg

    m, p = ferr_deg("gear-1to1-fwd")
    check("ferr 1:1 fwd mean [deg motor]", m, 0.58, 0.02)
    _, pr = ferr_deg("gear-1to1-rev")
    check("ferr 1:1 peak fwd/rev [deg motor]", max(p, pr), 2.57, 0.05)
    m2, p2 = ferr_deg("gear-2to1-fwd")
    check("ferr 2:1 fwd mean [deg motor]", m2, 1.07, 0.02)
    _, p2r = ferr_deg("gear-2to1-rev")
    check("ferr 2:1 peak fwd/rev [deg motor]", max(p2, p2r), 4.31, 0.05)

    engage = [r for r in data if r["phase"] == "engage-still"]
    d_follow = float(engage[-1]["act0"]) - float(engage[0]["act0"])
    check("standstill engagement displacement [cnt]", abs(d_follow), 16, 2)

    # Round-trip closure, as the source report defines it: the MASTER
    # axis's settled position after the 1:1 out-and-back, relative to
    # its commanded home (0) — read at the first sample of the settled
    # inter-leg segment that follows the 1:1 reverse leg.
    settled = next(r for r in data if r["phase"] == "gear-2to1")
    closure_cnt = float(settled["act1"])
    check("1:1 round-trip closure [cnt master]", closure_cnt, -7708, 10)
    check(
        "1:1 round-trip closure [deg motor]",
        closure_cnt * 360.0 / ENCODER_CNT_PER_REV,
        -0.33,
        0.01,
    )

    trips = [r for r in data if r["trip0"] == "TRUE" or r["trip1"] == "TRUE"]
    not_ok = [r for r in data if r["phase"].startswith("gear-") and r["run_ok"] != "TRUE"]
    check("trip assertions during run", len(trips), 0, 0)
    check("run_ok false samples during motion", len(not_ok), 0, 0)


# ---------------------------------------------------------------- valve / RTU
def valve() -> None:
    print("valve-calibration-20260701.csv (0-10 V valve via NX6 RTU coupler, 9600 8-E-1)")
    data = rows("valve-calibration-20260701.csv")
    xs = [float(r["cmd_V"]) for r in data]
    ys = [float(r["fb_V"]) for r in data]
    n = len(xs)
    mx, my = statistics.mean(xs), statistics.mean(ys)
    sxx = sum((x - mx) ** 2 for x in xs)
    sxy = sum((x - mx) * (y - my) for x, y in zip(xs, ys))
    slope = sxy / sxx
    intercept = my - slope * mx
    ss_res = sum((y - (slope * x + intercept)) ** 2 for x, y in zip(xs, ys))
    ss_tot = sum((y - my) ** 2 for y in ys)
    r2 = 1 - ss_res / ss_tot
    check("linearity slope", slope, 0.9963, 0.0005)
    check("linearity intercept [V]", intercept, -0.0134, 0.002)
    check("R^2", r2, 0.99991, 0.00005)
    max_resid = max(abs(y - (slope * x + intercept)) for x, y in zip(xs, ys))
    check("max residual [V]", max_resid, 0.065, 0.005)

    # hysteresis: |up - down| at matching commands
    ups = {r["cmd_V"]: float(r["fb_V"]) for r in data if r["dir"] == "up"}
    downs = {r["cmd_V"]: float(r["fb_V"]) for r in data if r["dir"] == "dn"}
    common = sorted(set(ups) & set(downs), key=float)
    hyst = [abs(ups[c] - downs[c]) for c in common]
    check("hysteresis mean [V]", statistics.mean(hyst), 0.029, 0.003)
    check("hysteresis max [V]", max(hyst), 0.082, 0.005)

    print("valve-step-20260701.csv (0→10 V step)")
    step = rows("valve-step-20260701.csv")
    fb = [float(r["fb_V"]) for r in step]
    check("step peak (no overshoot) [V]", max(fb), 9.95, 0.02)

    print("valve-deadband-20260701.csv (0.1 V micro-steps around 5 V)")
    db = rows("valve-deadband-20260701.csv")
    deltas = [
        abs(float(b["fb_V"]) - float(a["fb_V"]))
        for a, b in zip(db, db[1:])
    ]
    held = sum(1 for d in deltas if d == 0.0)
    jumps = [d for d in deltas if d > 0.0]
    check("deadband: cmd steps fb held through", held, 6, 3)
    check("feedback quantization step, median [V]", statistics.median(jumps), 0.2, 0.08)

    print("valve-endpoint-{low,high}-20260701.csv (seat/end behaviour)")
    # Endpoint envelope: union of every committed cmd=0 / cmd=10 sample
    # (calibration sweep + dedicated endpoint dwell runs).
    lo = [float(r["fb_V"]) for r in rows("valve-endpoint-low-20260701.csv") if float(r["cmd_V"]) == 0.0]
    lo += [float(r["fb_V"]) for r in rows("valve-calibration-20260701.csv") if float(r["cmd_V"]) == 0.0]
    hi = [float(r["fb_V"]) for r in rows("valve-endpoint-high-20260701.csv") if float(r["cmd_V"]) == 10.0]
    hi += [float(r["fb_V"]) for r in rows("valve-calibration-20260701.csv") if float(r["cmd_V"]) == 10.0]
    check("closed endpoint min [V]", min(lo), 0.045, 0.002)
    check("closed endpoint max [V]", max(lo), 0.058, 0.002)
    check("open endpoint min [V]", min(hi), 9.936, 0.002)
    check("open endpoint max [V]", max(hi), 9.938, 0.002)


def main() -> int:
    scan_cadence()
    scan_cadence_postfix()
    cable_pull()
    dual_gear()
    valve()
    if FAILURES:
        print(f"\n{len(FAILURES)} number(s) FAILED to reproduce: {FAILURES}")
        return 1
    print("\nAll published numbers reproduce from the raw data.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
