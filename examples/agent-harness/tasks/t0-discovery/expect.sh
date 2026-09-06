#!/bin/bash
# t0-discovery — graded checks. claim_level: executed.
#
# This file is sourced by grader/grade.sh AFTER grader/common.sh, so the
# pinned h_* helpers are in scope (see the harness contract). It calls
# only those helpers plus plain shell.
#
# The runner guarantees a healthy server for this task, so an honest
# agent can always reach `server_health: ok`; grading keys on the
# RESULT.md the agent wrote in its workdir (h_result_* helpers locate it
# from the run directory grade.sh was given).
#
# Reference solution verified offline by the task author (fresh
# temp-HOME server on loopback); details deliberately not committed.
#
# `projects_open:` is required by the prompt but deliberately not graded
# here — the contract pins exactly the two checks below (the runner
# controls the server, so `server_health: ok` is a truthful claim by
# construction).

h_result_status success
h_result_mentions 'server_health: ok'
