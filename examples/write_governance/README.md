# Write governance

No devices or hardware. Open this project and run
`cs sim run examples/write_governance/scenarios/clamp.toml` on a loopback
server. The scenario writes above, below and inside the declared range,
then checks the value consumed by the program. Removing governance makes
the first assertion fail. Direct writes to `observed` must return HTTP 403;
that denial and the response's applied value are covered by bridge tests.
