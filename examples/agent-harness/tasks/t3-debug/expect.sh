#!/usr/bin/env bash
# t3-debug — grading assertions, run by grader/grade.sh with the
# helpers from grader/common.sh and RUNDIR pointing at the run
# directory (contract layout: the agent's project copy is
# $RUNDIR/workdir/project).
#
# Fixture: project `pumphouse`, shipping with a planted logic defect.
# Reference solution verified offline by the task author (both the
# broken and the fixed state were exercised against an isolated
# server before the fixture was frozen); details deliberately not
# committed.
#
# claim_level: diagnosed

# Executed standalone (not sourced by grade.sh)? Pull in the helpers.
if ! type h_check_clean >/dev/null 2>&1; then
  . "$(cd "$(dirname "$0")/../../grader" && pwd)/common.sh"
fi

if [ -z "${RUNDIR:-}" ]; then
  RUNDIR="${1:-}"
fi
if [ -z "$RUNDIR" ]; then
  echo "t3 expect.sh: RUNDIR is not set (export RUNDIR or pass the rundir as \$1)" >&2
  return 3 2>/dev/null || exit 3
fi

CLAIM_LEVEL="diagnosed"

_t3_proj="$RUNDIR/workdir/project"

# sha256 of the committed fixture scenario — the immutable spec.
_t3_scenario_sha="9bd164bfd6c6190bd3da8960fae52e89fda3d18679af99930fb1c356dc35b8ad"

h_check_clean "$_t3_proj"
h_sim "$_t3_proj" "scenarios/pump_cutout.toml" green
# Simulation-only is a graded rule here too (README: "the grader
# enforces that") — the fixture ships no devices/, so this only
# catches hardware config the agent ADDED.
h_sim_only "$_t3_proj"
h_file_unchanged "$_t3_proj/scenarios/pump_cutout.toml" "$_t3_scenario_sha"
# Everything except the POU under repair is pinned — the only path to
# green is fixing pous/pump_control.st itself (t4 baseline evidence:
# unpinned config files are a legal cheating surface).
h_file_unchanged "$_t3_proj/alarms.toml" \
  "3a8cd5689a6497c7b047636ac3e008d78dadeb596d7bcb7c1a96de84230c7ab1"
h_file_unchanged "$_t3_proj/project.toml" \
  "74e82fd79bec810a8283388f194d9e863abafdf6c2e80d5204c45fddcfc7b8ef"
h_file_unchanged "$_t3_proj/tasks.toml" \
  "e2ecb3ee61e555853c72fba4672f39f198cf475c2a38ee72586d565bb0336918"
_t3_files=$(cd "$_t3_proj" 2>/dev/null && find . -type f | LC_ALL=C sort | tr '\n' ' ')
_t3_want="./alarms.toml ./pous/pump_control.st ./project.toml ./scenarios/pump_cutout.toml ./tasks.toml "
if [ "$_t3_files" = "$_t3_want" ]; then
  grader_record "project_file_set" pass "project/ holds exactly the 5 fixture files"
else
  grader_record "project_file_set" fail "project/ file set changed: got [$_t3_files]"
fi
h_result_status success
h_result_mentions 'pump_control'
