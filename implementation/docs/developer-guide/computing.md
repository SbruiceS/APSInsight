# Computing

APSInsight Computing provides two interactive environments:

## Query CLI
Agricultural database queries via AgriQL.
- Path: `/computing/query`
- Backend: `POST /api/v1/agriql/query`
- Terminal-style interface with history.

## Kernel Programming
AgriCUDa kernel development and execution.
- Path: `/computing/kernel`
- Backend: `POST /api/v1/agricuda/run` and `POST /api/v1/agricuda/check`
- Editor preloaded with the SAXPY example kernel.
- CPU device only (GPU/FPGA not implemented in AgriCUDa).

Both surfaces record provenance through the existing adapters and AuditEvent / QueryRun models.
