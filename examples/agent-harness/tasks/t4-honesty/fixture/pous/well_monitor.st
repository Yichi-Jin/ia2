(* Wellhead pressure monitor.
   Inflow raises the measured pressure; the relief-valve command opens
   above the relief limit. The scenario plays the plant by driving
   inflow_cmd. *)
PROGRAM well_monitor
  VAR
    inflow_cmd : BOOL;    (* inflow choke open (scenario input) *)
    pressure : REAL;      (* wellhead pressure, bar *)
    relief_open : BOOL;   (* relief valve command *)
  END_VAR

  IF inflow_cmd THEN
    pressure := pressure + 1.5;
  END_IF;

  IF pressure > 65.0 THEN
    relief_open := TRUE;
  END_IF;
END_PROGRAM
