#!/bin/bash
# t2-spec — graded checks. claim_level: generated.
#
# This file is sourced by grader/grade.sh AFTER grader/common.sh, so the
# pinned h_* helpers are in scope (see the harness contract). It calls
# only those helpers plus plain grep/test. grade.sh received the run
# directory as its first argument; common.sh conventionally exposes it
# as RUNDIR (agent cwd = $RUNDIR/workdir). The fallbacks below keep this
# file working whether the grader exports WORKDIR, RUNDIR, or runs with
# cwd at the run directory.
#
# Reference solution (including its falsifiability proof) verified
# offline by the task author against an isolated temp-HOME server on
# loopback; details deliberately not committed.
#
# No fixture/ dir: the agent authors everything; grading re-executes the
# proof against the project it left at workdir/project.

_wd="${WORKDIR:-${RUNDIR:-$PWD}/workdir}"

h_check_clean "$_wd/project"
h_sim "$_wd/project" scenarios/interlock.toml green
h_sim_only "$_wd/project"

# Structural checks on the agent-authored proof scenario (prescribed
# names, comment-stripped line greps — deterministic, no TOML
# parsing): a green re-run of a VACUOUS scenario proves nothing, so
# interlock.toml must actually contain the interlock proof:
#   1. a set step driving level_a to a value >= 80 (the high limit),
#   2. AFTER it, an expect step on pump with op is_false (the
#      pump-must-stop assertion),
#   3. an expect_alarm step on the prescribed id high_level_a.
_t2_body=$(sed 's/#.*$//' "$_wd/project/scenarios/interlock.toml" 2>/dev/null)
_t2_setline=$(printf '%s\n' "$_t2_body" | awk '
  /set[[:space:]]*=/ && /var[[:space:]]*=[[:space:]]*"level_a"/ {
    if (match($0, /value[[:space:]]*=[[:space:]]*[0-9.]+/)) {
      v = substr($0, RSTART, RLENGTH)
      sub(/^value[[:space:]]*=[[:space:]]*/, "", v)
      if (v + 0 >= 80) { print NR; exit }
    }
  }')
_t2_stopline=""
if [ -n "$_t2_setline" ]; then
  _t2_stopline=$(printf '%s\n' "$_t2_body" | awk -v n="$_t2_setline" '
    NR > n && /expect[[:space:]]*=/ && /var[[:space:]]*=[[:space:]]*"pump"/ && /"is_false"/ { print NR; exit }')
fi
if [ -n "$_t2_setline" ] && [ -n "$_t2_stopline" ]; then
  grader_record "scenario_pump_must_stop" pass \
    "interlock.toml drives level_a >= 80 (line $_t2_setline) and then asserts pump is_false (line $_t2_stopline)"
else
  grader_record "scenario_pump_must_stop" fail \
    "interlock.toml lacks a set of level_a to >= 80 followed by an expect of pump is_false — the pump-must-stop assertion is not proven"
fi
if printf '%s\n' "$_t2_body" |
  grep -E -q 'expect_alarm[[:space:]]*=.*id[[:space:]]*=[[:space:]]*"high_level_a"'; then
  grader_record "scenario_alarm_asserted" pass \
    "interlock.toml asserts expect_alarm on high_level_a"
else
  grader_record "scenario_alarm_asserted" fail \
    "interlock.toml has no expect_alarm step on the prescribed id 'high_level_a'"
fi

# Prescribed alarm id must be DECLARED as a real TOML key line in the
# project's alarms.toml — a comment mentioning the id must not count
# (recorded as a real check row so a miss grades FAIL, the agent's
# fault, not blocked/infra).
if grep -E -q '^[[:space:]]*id[[:space:]]*=[[:space:]]*"high_level_a"' \
  "$_wd/project/alarms.toml" 2>/dev/null; then
  grader_record "alarm_id_present" pass "alarms.toml declares id = \"high_level_a\""
else
  grader_record "alarm_id_present" fail \
    "project/alarms.toml has no id = \"high_level_a\" key line (comments do not count)"
fi

h_result_status success
