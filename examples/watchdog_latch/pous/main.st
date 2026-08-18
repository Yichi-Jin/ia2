(* Bench fixture for the scan-watchdog latch.

   Two ways to make the scans overrun, one per environment:

   - sim: `scenarios/latch.toml` uses the `inject` step — deterministic,
     host-speed independent, no burn needed. burn_n stays 0.
   - bench: force the burn live (`cs runtime force burn_n 200000
     --edge <edge>`) so every scan overruns for real, HTTP-free once
     armed. Five in a row trip the watchdog, which calls enter_failsafe
     (zeroing the output PDI) and latches the output phase off. What the
     bench answers is whether those zeros STAY on the wire.

   burn_n defaults to 0 (harmless scan) so the project deploys healthy
   and the trip is an explicit operator/scenario action, not a startup
   race.

   Safety: control_word is only ever driven to 16#0006 (Shutdown), which
   parks a CiA402 drive at Ready-to-switch-on. Operation Enabled is never
   commanded, so no axis can move during this test. The observable is the
   drive's statusword, not motion.

   When forcing burn_n, raise it until scan_count stops tracking wall
   clock, i.e. until the scans actually overrun on YOUR host. *)
PROGRAM main
  VAR
    burn_n       : DINT := 0;
    idx          : DINT;
    sink         : DINT;
    scans        : DINT;
    status_word  : WORD;
    control_word : WORD;
  END_VAR

  scans := scans + 1;

  (* Deliberate overrun. Pure arithmetic so the cost is predictable and
     nothing outside the VM is touched. *)
  FOR idx := 1 TO burn_n DO
    sink := sink + idx;
  END_FOR;

  (* Hold at Ready-to-switch-on. 16#0006 = Enable voltage + Quick stop
     with Switch on cleared: energised, not enabled, cannot move. *)
  control_word := 16#0006;
END_PROGRAM
