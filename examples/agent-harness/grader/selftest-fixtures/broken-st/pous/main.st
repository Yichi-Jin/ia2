(* Grader selftest fixture — INTENTIONALLY BROKEN Structured Text.
   The IF is never closed, so `cs check` and `cs project check` must
   both report diagnostics (exit 1). h_check_clean must catch this. *)
PROGRAM main
  VAR
    run_cmd : BOOL;
    level : REAL;
  END_VAR
  IF run_cmd THEN
    level := level + 1.0;
END_PROGRAM
