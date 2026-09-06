(* Grader selftest fixture — minimal tank integrator, derived from
   examples/sim_smoke. The scenario plays the plant by writing
   inlet_cmd; level integrates while it holds. *)
PROGRAM main
  VAR
    inlet_cmd : BOOL;
    level : REAL;
    overflow : BOOL;
  END_VAR
  IF inlet_cmd THEN
    level := level + 0.5;
  END_IF;
  IF level > 80.0 THEN
    overflow := TRUE;
  END_IF;
END_PROGRAM
