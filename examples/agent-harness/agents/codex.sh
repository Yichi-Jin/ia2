#!/bin/bash
# Adapter: OpenAI Codex CLI (`codex`), non-interactive.
#
# Contract with run.sh: stdout line 1 is `HARNESS_TOOL_VERSION: ...`;
# runs in the task workdir with HARNESS_PROMPT / HARNESS_SERVER_URL /
# HARNESS_TIMEOUT_SECS set; combined output becomes the transcript.
# Exit 3 means "blocked" (tool missing), not a task failure.
#
# TODO: verify the exact non-interactive invocation against current
# Codex CLI docs before first use — the sandbox/approval flag names have
# changed between releases; the flags below match the last-known surface
# (`codex exec`, `--skip-git-repo-check` because the workdir is not a
# git repo, and the sandbox bypass as the analog of Claude Code's
# skip-permissions — acceptable only because the workdir is an isolated
# throwaway and `cs` on PATH is pinned to the harness's loopback server).

set -u

if ! command -v codex >/dev/null 2>&1; then
  echo "codex CLI not found on PATH — install it first" >&2
  echo "  (npm install -g @openai/codex) — blocked, not a task failure" >&2
  exit 3
fi

echo "HARNESS_TOOL_VERSION: $(codex --version 2>/dev/null | head -n 1)"

TIMEOUT_SECS="${HARNESS_TIMEOUT_SECS:-1200}"

# Time budget with whole-tree teardown: the tool runs as the leader of
# its own process group; on timeout the GROUP gets TERM, a short grace,
# then KILL, so children the tool spawned cannot outlive the budget or
# keep the workdir open. One perl implementation everywhere (macOS
# ships no GNU `timeout`) so the behavior is deterministic across
# machines. Exit: child status propagated; signal deaths map to 128+N
# (timeout => 143).
exec perl -e '
  my $secs = shift @ARGV;
  my $pid  = fork;
  die "fork failed: $!\n" unless defined $pid;
  if ($pid == 0) {
    setpgrp(0, 0);                 # own group => timeout kills the tree
    exec @ARGV or die "exec failed: $!\n";
  }
  $SIG{ALRM} = sub {
    kill "TERM", -$pid;            # polite stop for the whole group
    sleep 5;                       # grace for orderly child shutdown
    kill "KILL", -$pid;            # hard stop for anything left
  };
  alarm $secs;
  my $r;
  for (;;) {
    $r = waitpid($pid, 0);
    last if $r == $pid or ($r == -1 and not $!{EINTR});
  }
  my $st = $?;
  exit(128 + ($st & 127)) if $st & 127;
  exit($st >> 8);
' "$TIMEOUT_SECS" \
  codex exec --skip-git-repo-check --dangerously-bypass-approvals-and-sandbox \
  "$(cat "$HARNESS_PROMPT")"
