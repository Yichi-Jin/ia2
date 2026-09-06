#!/bin/bash
# t1-guided — graded checks. claim_level: executed.
#
# This file is sourced by grader/grade.sh AFTER grader/common.sh, so the
# pinned h_* helpers are in scope (see the harness contract). It calls
# only those helpers plus plain shell. grade.sh received the run
# directory as its first argument; common.sh conventionally exposes it
# as RUNDIR (agent cwd = $RUNDIR/workdir). The fallbacks below keep this
# file working whether the grader exports WORKDIR, RUNDIR, or runs with
# cwd at the run directory.
#
# Reference solution verified offline by the task author against an
# isolated temp-HOME server on loopback; details deliberately not
# committed.
#
# No fixture/ dir: the agent authors everything; grading re-executes the
# proof against the project it left at workdir/project.

_wd="${WORKDIR:-${RUNDIR:-$PWD}/workdir}"

h_check_clean "$_wd/project"
h_sim "$_wd/project" scenarios/toggle.toml green
h_sim_only "$_wd/project"

# Structural check on the agent-authored proof scenario (prescribed
# names, comment-stripped grep — deterministic, no TOML parsing): a
# green re-run of a VACUOUS scenario proves nothing, so toggle.toml
# must actually assert BOTH lamp transitions — an expect step on var
# "lamp" with op is_true AND one with is_false.
_t1_body=$(sed 's/#.*$//' "$_wd/project/scenarios/toggle.toml" 2>/dev/null)
if printf '%s\n' "$_t1_body" |
  grep -E -q 'expect[[:space:]]*=.*var[[:space:]]*=[[:space:]]*"lamp".*"is_true"' &&
  printf '%s\n' "$_t1_body" |
  grep -E -q 'expect[[:space:]]*=.*var[[:space:]]*=[[:space:]]*"lamp".*"is_false"'; then
  grader_record "scenario_asserts_both_transitions" pass \
    "toggle.toml holds expect steps on lamp for is_true AND is_false"
else
  grader_record "scenario_asserts_both_transitions" fail \
    "toggle.toml lacks a non-comment expect step on lamp for is_true AND one for is_false — the scenario does not prove the toggle"
fi

h_result_status success
