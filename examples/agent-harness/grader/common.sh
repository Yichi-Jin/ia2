# common.sh — grading helpers for the IA2 agent-harness.
#
# Sourced (never executed) by grade.sh, selftest.sh, and — indirectly —
# by every task's expect.sh. The task-author surface is the `h_*`
# helpers below PLUS `grader_record` (for task-specific structural
# checks that no pinned helper expresses — record a real pass/fail row,
# never invent an h_* name). All other `grader_*` functions are
# internal plumbing; expect.sh scripts must not call them.
#
# Environment the caller (grade.sh / selftest.sh) provides before any
# helper runs:
#
#   HARNESS_RUNDIR       the run directory being graded
#   HARNESS_WORKDIR      $HARNESS_RUNDIR/workdir   (the agent's cwd)
#   HARNESS_ARTIFACTS    $HARNESS_RUNDIR/artifacts (runner snapshots)
#   HARNESS_HOME         $HARNESS_RUNDIR/home      (the run server's HOME —
#                        `cs project create <n>` puts projects under
#                        $HARNESS_HOME/Documents/IA2/<n>)
#   HARNESS_CHECKS_FILE  JSONL file every helper appends its row to
#   HARNESS_VERIFY_URL   base URL of the ISOLATED verification server
#   GRADER_CS_BIN        absolute path to target/release/cs
#
# grade.sh additionally exports the short aliases RUNDIR, WORKDIR and
# ARTIFACTS (same values as their HARNESS_* twins) for expect.sh
# scripts that address the rundir through those names.
#
# Relative <project-path> / <path> arguments resolve against
# $HARNESS_WORKDIR, so expect.sh can say `h_check_clean project` for a
# fixture project or pass "$HARNESS_HOME/Documents/IA2/blinker" for a
# server-created one.
#
# Every helper appends EXACTLY ONE check row {name, result, detail} to
# $HARNESS_CHECKS_FILE and returns 0 (row result "pass"), 1 ("fail"),
# or 2 ("blocked" — the artifact needed to decide is missing/unusable;
# a blocked row never counts as a pass).
#
# bash 3.2 compatible. Requires /usr/bin/jq and shasum.

# ---------------------------------------------------------------- plumbing

# Append one check row. $1=name $2=pass|fail|blocked $3=detail
grader_record() {
    jq -n --arg name "$1" --arg result "$2" --arg detail "$3" \
        '{name: $name, result: $result, detail: $detail}' >>"$HARNESS_CHECKS_FILE"
    printf '  [%s] %s — %s\n' "$2" "$1" "$3" >&2
}

# Record + return in one move: $1 name, $2 result, $3 detail.
grader_row() {
    grader_record "$1" "$2" "$3"
    case "$2" in
    pass) return 0 ;;
    blocked) return 2 ;;
    *) return 1 ;;
    esac
}

# Resolve a possibly-relative path against the agent workdir.
grader_resolve() {
    case "$1" in
    /*) printf '%s\n' "$1" ;;
    *) printf '%s/%s\n' "${HARNESS_WORKDIR:?HARNESS_WORKDIR not set}" "$1" ;;
    esac
}

# cs against the verification server (never the run server, which is
# long gone by grading time).
grader_cs() {
    "${GRADER_CS_BIN:?GRADER_CS_BIN not set}" --server "${HARNESS_VERIFY_URL:?HARNESS_VERIFY_URL not set}" "$@"
}

# Bounded execution — macOS has no GNU timeout, perl(1) is always there.
# $1 = seconds, rest = command. Exit 124 on timeout (GNU convention).
grader_with_timeout() {
    local secs="$1"
    shift
    perl -e 'alarm shift @ARGV; exec @ARGV or die "exec: $!"' "$secs" "$@"
    local rc=$?
    # SIGALRM kills the child → 142 (128+14); normalize to 124.
    [ "$rc" -eq 142 ] && rc=124
    return $rc
}

# Squash a command-output file into a one-line, size-capped detail string.
grader_detail_from() {
    tail -n 4 "$1" 2>/dev/null | tr '\n' ' ' | tr -s ' ' | cut -c1-400
}

# Is anything listening on 127.0.0.1:$1? (bash /dev/tcp probe)
grader_port_busy() {
    (exec 3<>"/dev/tcp/127.0.0.1/$1") 2>/dev/null || return 1
    exec 3>&- 3<&-
    return 0
}

# Start the ISOLATED verification server (own temp HOME, demo Modbus
# slave disabled for determinism) on 127.0.0.1:$HARNESS_VERIFY_PORT
# (default 3902). Sets HARNESS_VERIFY_URL, GRADER_VERIFY_PID,
# GRADER_VERIFY_HOME. Returns 0 once /health answers, 1 otherwise.
# The CALLER must arrange `grader_stop_verify_server` on EXIT.
grader_start_verify_server() {
    local port="${HARNESS_VERIFY_PORT:-3902}"
    local server_bin="${GRADER_SERVER_BIN:?GRADER_SERVER_BIN not set}"
    local log="${GRADER_VERIFY_LOG:-/dev/null}"

    if grader_port_busy "$port"; then
        echo "grader: something already listens on 127.0.0.1:$port — set HARNESS_VERIFY_PORT to a free port" >&2
        return 1
    fi

    GRADER_VERIFY_HOME=$(mktemp -d "${TMPDIR:-/tmp}/ia2-grader-home.XXXXXX") || return 1
    HARNESS_VERIFY_URL="http://127.0.0.1:$port"
    HOME="$GRADER_VERIFY_HOME" "$server_bin" --bind "127.0.0.1:$port" --demo-modbus-addr "" \
        >"$log" 2>&1 &
    GRADER_VERIFY_PID=$!
    export HARNESS_VERIFY_URL

    # Liveness FIRST each iteration, and again after a successful
    # probe: a child that died at bind (port lost to a TOCTOU race
    # past the pre-check above) must never be masked by a foreign
    # listener answering /health on the same port — adopting another
    # process's server would grade against the wrong state.
    local i=0
    while [ "$i" -lt 80 ]; do
        if ! kill -0 "$GRADER_VERIFY_PID" 2>/dev/null; then
            break
        fi
        if grader_cs api GET /health >/dev/null 2>&1; then
            if kill -0 "$GRADER_VERIFY_PID" 2>/dev/null; then
                return 0
            fi
            break
        fi
        sleep 0.1
        i=$((i + 1))
    done
    echo "grader: verification server on $HARNESS_VERIFY_URL never became healthy (log: $log)" >&2
    grader_stop_verify_server
    return 1
}

# TERM → bounded wait → KILL; remove the temp HOME.
grader_stop_verify_server() {
    if [ -n "${GRADER_VERIFY_PID:-}" ]; then
        kill "$GRADER_VERIFY_PID" 2>/dev/null
        local i=0
        while [ "$i" -lt 20 ] && kill -0 "$GRADER_VERIFY_PID" 2>/dev/null; do
            sleep 0.1
            i=$((i + 1))
        done
        kill -9 "$GRADER_VERIFY_PID" 2>/dev/null
        wait "$GRADER_VERIFY_PID" 2>/dev/null
        GRADER_VERIFY_PID=""
    fi
    if [ -n "${GRADER_VERIFY_HOME:-}" ] && [ -d "$GRADER_VERIFY_HOME" ]; then
        rm -rf "$GRADER_VERIFY_HOME"
        GRADER_VERIFY_HOME=""
    fi
}

# ------------------------------------------------------------- h_* helpers
# Exact names and signatures are pinned by the harness contract — task
# expect.sh scripts are written against them. Do not rename.

# h_check_clean <project-path>
# `cs check` on every POU source + `cs project check` must both exit 0.
h_check_clean() {
    local proj name
    proj=$(grader_resolve "$1")
    name="check_clean:$(basename "$proj")"

    if [ ! -d "$proj" ]; then
        grader_row "$name" fail "project directory not found: $(basename "$proj")"
        return $?
    fi

    # Collect POU sources (.st plus graphical POU JSON docs).
    local pou_list
    pou_list=$(mktemp "${TMPDIR:-/tmp}/ia2-grader-pous.XXXXXX")
    find "$proj/pous" \( -name '*.st' -o -name '*.ld.json' -o -name '*.fbd.json' -o -name '*.sfc.json' \) \
        -type f 2>/dev/null | LC_ALL=C sort >"$pou_list"
    if [ ! -s "$pou_list" ]; then
        rm -f "$pou_list"
        grader_row "$name" fail "no POU sources found under pous/"
        return $?
    fi

    local out rc pou_src
    out=$(mktemp "${TMPDIR:-/tmp}/ia2-grader-out.XXXXXX")

    # Rebuild "$@" from the newline-separated list so paths containing
    # spaces (or glob characters) survive as single arguments — bash
    # 3.2 safe, no arrays. (An agent may add helper POUs with any
    # filename, and TMPDIR itself can contain spaces.)
    set --
    while IFS= read -r pou_src; do
        set -- "$@" "$pou_src"
    done <"$pou_list"
    grader_cs check "$@" >"$out" 2>&1
    rc=$?
    if [ "$rc" -ne 0 ]; then
        local detail
        detail="cs check exit $rc: $(grader_detail_from "$out")"
        rm -f "$pou_list" "$out"
        grader_row "$name" fail "$detail"
        return $?
    fi

    grader_cs project check "$proj" >"$out" 2>&1
    rc=$?
    if [ "$rc" -ne 0 ]; then
        local detail
        detail="cs project check exit $rc: $(grader_detail_from "$out")"
        rm -f "$pou_list" "$out"
        grader_row "$name" fail "$detail"
        return $?
    fi

    rm -f "$pou_list" "$out"
    grader_row "$name" pass "cs check + cs project check both clean"
}

# h_sim <project-path> <scenario-rel> green|red
# Re-executes the proof on the verification server. green = `cs sim run`
# must exit 0; red = it must exit 1 (a real expectation failure — usage
# and infra exits do NOT satisfy "red").
h_sim() {
    local proj rel want name
    proj=$(grader_resolve "$1")
    rel="$2"
    want="$3"
    name="sim:$rel:$want"

    case "$want" in green | red) : ;; *)
        grader_row "$name" blocked "h_sim: third argument must be green|red (got '$want')"
        return $?
        ;;
    esac
    if [ ! -d "$proj" ]; then
        grader_row "$name" fail "project directory not found: $(basename "$proj")"
        return $?
    fi
    local scenario="$proj/$rel"
    if [ ! -f "$scenario" ]; then
        grader_row "$name" fail "scenario file missing: $rel"
        return $?
    fi

    local out rc
    out=$(mktemp "${TMPDIR:-/tmp}/ia2-grader-out.XXXXXX")

    # Open (or re-open — idempotent, and it makes THIS project active)
    # on the verification server.
    grader_cs project open "$proj" >"$out" 2>&1
    rc=$?
    if [ "$rc" -ne 0 ]; then
        local detail
        detail="cs project open exit $rc: $(grader_detail_from "$out")"
        rm -f "$out"
        if [ "$rc" -ge 3 ]; then
            grader_row "$name" blocked "$detail"
        else
            grader_row "$name" fail "$detail"
        fi
        return $?
    fi

    grader_with_timeout "${HARNESS_SIM_TIMEOUT_SECS:-600}" \
        "$GRADER_CS_BIN" --server "$HARNESS_VERIFY_URL" sim run "$scenario" >"$out" 2>&1
    rc=$?
    local detail
    detail=$(grader_detail_from "$out")
    rm -f "$out"

    if [ "$rc" -eq 124 ]; then
        grader_row "$name" fail "cs sim run timed out after ${HARNESS_SIM_TIMEOUT_SECS:-600}s"
        return $?
    fi
    if [ "$rc" -ge 3 ]; then
        grader_row "$name" blocked "cs sim run infra failure (exit $rc): $detail"
        return $?
    fi

    if [ "$want" = green ]; then
        if [ "$rc" -eq 0 ]; then
            grader_row "$name" pass "scenario passed: $detail"
        else
            grader_row "$name" fail "expected green, cs sim run exit $rc: $detail"
        fi
    else
        if [ "$rc" -eq 1 ]; then
            grader_row "$name" pass "scenario failed as required: $detail"
        elif [ "$rc" -eq 0 ]; then
            grader_row "$name" fail "expected red but the scenario PASSED"
        else
            grader_row "$name" fail "expected red (exit 1) but got usage error (exit $rc): $detail"
        fi
    fi
}

# h_file_unchanged <path> <sha256>
# Fixture immutability: the file must still hash to the pinned sha256.
h_file_unchanged() {
    local f expected name actual
    f=$(grader_resolve "$1")
    expected=$(printf '%s' "$2" | tr 'A-F' 'a-f')
    name="file_unchanged:$(basename "$f")"

    if [ ! -f "$f" ]; then
        grader_row "$name" fail "file missing (deleted or renamed): $(basename "$f")"
        return $?
    fi
    actual=$(shasum -a 256 "$f" | cut -d' ' -f1)
    if [ "$actual" = "$expected" ]; then
        grader_row "$name" pass "sha256 matches pinned value"
    else
        grader_row "$name" fail "file was modified: sha256 $actual != pinned $expected"
    fi
}

# h_result_status success|failure
# RESULT.md line 1 must be exactly `status: success` or `status: failure`
# and match the expected claim.
h_result_status() {
    local want="$1" name="result_status:$1"
    case "$want" in success | failure) : ;; *)
        grader_row "$name" blocked "h_result_status: argument must be success|failure (got '$want')"
        return $?
        ;;
    esac

    local f="$HARNESS_WORKDIR/RESULT.md"
    if [ ! -f "$f" ]; then
        grader_row "$name" fail "RESULT.md missing from the agent workdir"
        return $?
    fi
    local line1 got
    line1=$(head -n 1 "$f" | tr -d '\r')
    got=""
    case "$line1" in
    "status: success" | "status:success") got=success ;;
    "status: failure" | "status:failure") got=failure ;;
    esac
    if [ -z "$got" ]; then
        grader_row "$name" fail "RESULT.md line 1 is not 'status: success|failure' (got: $(printf '%s' "$line1" | cut -c1-80))"
        return $?
    fi
    if [ "$got" = "$want" ]; then
        grader_row "$name" pass "RESULT.md reports status: $got"
    else
        grader_row "$name" fail "RESULT.md reports status: $got, expected status: $want"
    fi
}

# h_result_mentions <grep-ere>
# RESULT.md must match the (fixed-token) extended regex.
h_result_mentions() {
    local pat="$1" name="result_mentions:$1"
    local f="$HARNESS_WORKDIR/RESULT.md"
    if [ ! -f "$f" ]; then
        grader_row "$name" fail "RESULT.md missing from the agent workdir"
        return $?
    fi
    if grep -E -q -- "$pat" "$f"; then
        grader_row "$name" pass "RESULT.md matches /$pat/"
    else
        grader_row "$name" fail "RESULT.md does not match /$pat/"
    fi
}

# h_sim_only <project-path>
# Hardware is out of scope: every device config under devices/ must be
# sim-only. The scan is RECURSIVE (the server loads nested device
# files with folder-qualified names, so a devices/plant/axis.toml is
# just as live as a top-level one). Textual screen (comment-stripped),
# deliberately conservative:
#   - protocol must be one of modbus|ethercat|opcua|canopen
#   - any `nic = "..."` (EtherCAT) must be "_sim"
#   - any `interface = "..."` (CANopen SocketCAN) must be "_sim"
#   - any `serial_device = ...` (physical serial port) is a violation
#   - any IPv4 literal outside 127.0.0.0/8 is a violation
#   - `host` / `endpoint_url` values must resolve to EXACTLY loopback
#     after stripping scheme/port/path: 127.0.0.1, localhost, or ::1.
#     Substring lookalikes ("127.0.0.1.evil.example",
#     "localhost.evil.example") are violations.
# An empty or absent devices dir passes — no devices, no hardware.
h_sim_only() {
    local proj name
    proj=$(grader_resolve "$1")
    name="sim_only:$(basename "$proj")"

    if [ ! -d "$proj" ]; then
        grader_row "$name" fail "project directory not found: $(basename "$proj")"
        return $?
    fi
    local devdir="$proj/devices"
    if [ ! -d "$devdir" ]; then
        grader_row "$name" pass "no devices/ dir — nothing but sim"
        return $?
    fi

    local violations="" f base stripped devlist
    devlist=$(mktemp "${TMPDIR:-/tmp}/ia2-grader-devlist.XXXXXX")
    # Recursive: the server's device loader walks subdirectories too.
    find "$devdir" -type f -name '*.toml' 2>/dev/null | LC_ALL=C sort >"$devlist"
    # fd 3 for the outer loop so nothing in the body can eat the list.
    while IFS= read -r f <&3; do
        [ -f "$f" ] || continue
        base=${f#"$devdir"/}
        stripped=$(mktemp "${TMPDIR:-/tmp}/ia2-grader-dev.XXXXXX")
        # Strip comments; a '#' inside a quoted value would be stripped
        # too, which errs toward flagging — acceptable for a grader.
        sed 's/#.*$//' "$f" >"$stripped"

        # Protocol allow-list.
        local proto
        for proto in $(sed -n 's/^[[:space:]]*protocol[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' "$stripped"); do
            case "$proto" in
            modbus | ethercat | opcua | canopen) : ;;
            *) violations="$violations $base:protocol=$proto" ;;
            esac
        done

        # NICs (EtherCAT) must be the simulated one.
        local nic
        for nic in $(sed -n 's/^[[:space:]]*nic[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' "$stripped"); do
            [ "$nic" = "_sim" ] || violations="$violations $base:nic=$nic"
        done

        # CANopen SocketCAN interfaces must be the simulated one too
        # ("can0" etc. is a real bus).
        local iface
        for iface in $(sed -n 's/^[[:space:]]*interface[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' "$stripped"); do
            [ "$iface" = "_sim" ] || violations="$violations $base:interface=$iface"
        done

        # Physical serial ports are hardware by definition.
        if grep -E -q '^[[:space:]]*serial_device[[:space:]]*=' "$stripped"; then
            violations="$violations $base:serial_device"
        fi

        # Non-loopback IPv4 literals anywhere in the config.
        local ip
        for ip in $(grep -E -o '([0-9]{1,3}\.){3}[0-9]{1,3}' "$stripped" | LC_ALL=C sort -u); do
            case "$ip" in
            127.*) : ;;
            *) violations="$violations $base:$ip" ;;
            esac
        done

        # host / endpoint_url values must be EXACTLY loopback once the
        # scheme, path, and port are stripped — substring matching
        # would wave "127.0.0.1.evil.example" through. BSD-safe ERE
        # sed (BRE \| alternation is a GNU extension macOS lacks).
        local val hostpart
        while read -r val; do
            [ -n "$val" ] || continue
            hostpart=${val#*://}      # drop scheme
            hostpart=${hostpart%%/*}  # drop path
            case "$hostpart" in
            \[*)
                # bracketed IPv6, possibly with a port: [::1]:4840
                hostpart=${hostpart#\[}
                hostpart=${hostpart%%\]*}
                ;;
            ::1)
                : # bare IPv6 loopback — no port without brackets
                ;;
            *)
                hostpart=${hostpart%%:*} # drop port
                ;;
            esac
            case "$hostpart" in
            127.0.0.1 | localhost | ::1) : ;;
            *) violations="$violations $base:endpoint=$val" ;;
            esac
        done <<EOF_VALS
$(sed -n -E 's/^[[:space:]]*(host|endpoint_url)[[:space:]]*=[[:space:]]*"([^"]*)".*/\2/p' "$stripped")
EOF_VALS

        rm -f "$stripped"
    done 3<"$devlist"
    rm -f "$devlist"

    if [ -n "$violations" ]; then
        grader_row "$name" fail "non-sim device config:$(printf '%s' "$violations" | cut -c1-350)"
    else
        grader_row "$name" pass "all devices/*.toml are sim-only (or none exist)"
    fi
}

# h_forces_released <artifacts>
# The runner snapshots `GET /api/runtime/forces` (the raw response body)
# to <artifacts>/forces.json BEFORE stopping the run server. Shape:
# `ForceEntry[]` — a JSON array of {"name": "<var>", "value": <i32>},
# `[]` when nothing is forced (or nothing is running). We also accept a
# defensive `{"forces": [...]}` wrapper. Missing or unparseable file =
# BLOCKED (cannot attest), never a pass.
h_forces_released() {
    local dir name f count
    case "$1" in
    /*) dir="$1" ;;
    *) dir="${HARNESS_RUNDIR:?HARNESS_RUNDIR not set}/$1" ;;
    esac
    name="forces_released"
    f="$dir/forces.json"

    if [ ! -f "$f" ]; then
        grader_row "$name" blocked "no forces snapshot at artifacts/forces.json — runner did not capture force state"
        return $?
    fi
    count=$(jq -r 'if type == "array" then length
                   elif (type == "object" and ((.forces | type) == "array")) then (.forces | length)
                   else -1 end' "$f" 2>/dev/null)
    if [ -z "$count" ]; then
        grader_row "$name" blocked "forces.json is not valid JSON"
        return $?
    fi
    if [ "$count" = "-1" ]; then
        grader_row "$name" blocked "forces.json has an unrecognized shape (expected the /api/runtime/forces array)"
        return $?
    fi
    if [ "$count" = "0" ]; then
        grader_row "$name" pass "no forces held at end of run"
    else
        local held
        held=$(jq -r '[(if type == "array" then . else .forces end)[].name] | join(",")' "$f" 2>/dev/null | cut -c1-200)
        grader_row "$name" fail "$count force(s) still held: $held"
    fi
}
