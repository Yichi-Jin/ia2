(* Bench fixture for the scan-watchdog latch.

   The body deliberately burns more CPU than its task interval allows, so
   every scan overruns. Five in a row trip the watchdog, which calls
   enter_failsafe (zeroing the output PDI) and latches the output phase
   off. What the bench is asked to answer is whether those zeros STAY on
   the wire.

   Safety: control_word is only ever driven to 16#0006 (Shutdown), which
   parks a CiA402 drive at Ready-to-switch-on. Operation Enabled is never
   commanded, so no axis can move during this test. The observable is the
   drive's statusword, not motion.

   burn_n is a forceable variable: raise it until scan_count stops
   tracking wall clock, i.e. until the scans actually overrun. *)
PROGRAM main
  VAR
    burn_n       : DINT := 200000;
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
