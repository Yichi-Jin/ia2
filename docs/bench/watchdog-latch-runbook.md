# Bench runbook — scan-watchdog latch (P1 gate)

Verifies on real hardware that a tripped scan watchdog holds outputs at
zero until an explicit restart. Fixture: `examples/watchdog_latch/`.

**Zero motion.** The fixture never commands Operation Enabled. Drives stay
at Ready-to-switch-on throughout; the verdict is read from the statusword.
Nothing on the bench needs to be mechanically prepared.

**Time:** ~40 min including the A/B comparison.

---

## 0 · Preconditions

| Check | Command | Expect |
|---|---|---|
| edge up | `ssh edge 'uptime'` | responds (key auth, no password) |
| EtherCAT NIC up | `ssh edge 'ip -br link show enp2s0'` | `UP` — if `DOWN`: `sudo ip link set enp2s0 up` |
| Drives on the chain | `ssh edge 'curl -s localhost:13001/discover'` | 2 × `InoSV660N`, `connected: true` |
| Rollback point noted | `ssh edge 'ls -la ~/ia2-versions/'` | note the `current` symlink target |

> Nobody else should be using the bench — this restarts `ia2`.

---

## 1 · Baseline: capture the healthy statusword

```bash
ssh edge 'curl -s localhost:13001/status' | python3 -m json.tool | grep -A1 status_word
```

Record the value. Expected `16#1631` = Ready to switch on
(bit0 RTSO, bit4 voltage enabled, bit5 quick-stop inactive, bit6 clear).

**If it is anything else, stop** and resolve that first — the test reads a
transition *from* this state.

---

## 2 · Deploy the fixture with the **pre-fix** runtime

The point of this step is to see the bug. Build from `main` (or any commit
before the latch), not from `fix/watchdog-latch`.

```bash
# on the dev host
cd ~/soft-PLC/ia2
git stash list                      # confirm the fix is NOT in the tree
git checkout main
cargo build --release -p ia2-runtime

# point the fixture at the real NIC
sed -i '' 's/nic = "_sim".*/nic = "enp2s0"/' examples/watchdog_latch/devices/servo.toml

# deploy (versions dir + symlink + restart — the standard edge pattern)
cs deploy --edge <edge> --project examples/watchdog_latch
```

Watch the trip land:

```bash
ssh edge 'journalctl -u ia2 -f | grep -iE "watchdog|overran|failsafe"'
```

Expect within a second or two:

```
scan overran its budget — sliding cadence forward ...
watchdog tripped — engaging failsafe ...
```

**Confirm the overrun is real** (otherwise the whole test is vacuous):

```bash
ssh edge 'curl -s localhost:13001/status' | python3 -c "
import json,sys; d=json.load(sys.stdin)
print('scan_count', d['scan_count'], 'uptime', d['uptime_secs'])
print('scans/s =', round(d['scan_count']/max(d['uptime_secs'],1)))
print('healthy 2ms would be ~500/s; far below that means the burn is working')"
```

If scans/s is still ~500, raise `burn_n` (README explains) and redeploy.

### Read the verdict — pre-fix

```bash
for i in $(seq 1 10); do
  ssh edge 'curl -s localhost:13001/status' \
    | python3 -c "import json,sys; d=json.load(sys.stdin); \
      v={x['name']:x['value'] for x in d['last_snapshot']['vars']}; \
      print(v.get('status_word'))"
  sleep 1
done
```

**Expected pre-fix failure mode:** the statusword either never leaves
`16#1631`, or flicks to Switch-on-disabled and **comes back**. Either way
the zeros did not hold.

Record what you actually see — including "no visible change", which is the
race the code review flagged (the zeros may live on the wire for ~1 frame
and never be sampled).

---

## 3 · Deploy the **fixed** runtime

```bash
cd ~/soft-PLC/ia2
git checkout fix/watchdog-latch
cargo build --release -p ia2-runtime
cs deploy --edge <edge> --project examples/watchdog_latch
```

Repeat the 10-sample loop from step 2.

### Pass criteria

| # | Criterion | How |
|---|---|---|
| P1 | Watchdog trips | `watchdog tripped` in journal |
| P2 | Statusword leaves Ready-to-switch-on | bit 6 set (Switch on disabled) |
| P3 | **It stays there** — 10/10 samples over ≥10 s | the loop above |
| P4 | Restart clears it | `sudo systemctl restart ia2` → statusword returns to `16#1631` |

P3 is the whole test. P4 proves the latch is a latch and not a permanent
brick.

---

## 4 · Optional — settle the one-frame race

The code review could not determine statically whether the zeros ever
reach the wire before being overwritten (pre-fix), because the scan thread
and the cyclic bus thread are both ~2 ms and unsynchronised.

```bash
ssh edge 'sudo timeout 20 tcpdump -i enp2s0 -w /tmp/wd.pcap ether proto 0x88a4'
scp edge:/tmp/wd.pcap .
```

Look for RxPDO frames whose controlword bytes are `00 00`. Counting them
answers "did the zeros ever go out, and for how many frames".

---

## 5 · Restore the bench

```bash
cd ~/soft-PLC/ia2
git checkout examples/watchdog_latch/devices/servo.toml   # nic back to "_sim"
cs deploy --edge <edge> --project <whatever the bench normally runs>
ssh edge 'curl -s localhost:13001/status | head -c 200'  # normal project back
```

Leaving the lab: shut the edge down the usual way (`ssh -t edge 'sudo shutdown -h now'`).

---

## Recording the result

Whatever happens, write the observed statusword sequences for both builds
into the PR description. A "we saw no change pre-fix" result is still a
result — it means the failure mode is the race rather than the rewrite,
and the latch is still the correct fix (it removes both).
