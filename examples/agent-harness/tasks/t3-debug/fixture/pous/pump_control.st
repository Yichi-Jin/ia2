(* Pumphouse transfer pump.
   Runs on operator request; the high-level cutout must stop it when
   the receiving tank level reaches high_limit. The scenario plays the
   plant by writing run_request and level directly. *)
PROGRAM pump_control
  VAR
    run_request : BOOL;   (* operator start request (scenario input) *)
    level : REAL;         (* receiving tank level, percent (scenario input) *)
    pump : BOOL;          (* pump contactor command *)
    high_limit : REAL := 80.0;
  END_VAR

  pump := run_request;

  (* High-level cutout: never pump into a tank at the limit. *)
  IF level <= high_limit THEN
    pump := FALSE;
  END_IF;
END_PROGRAM
