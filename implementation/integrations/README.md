# Integrations

## AgriQL
Agricultural database query language (SbruiceS/AgriQL, engine 3.10.1).
Endpoints under /api/v1/agriql/. Real Python client when package present; explicit test double otherwise.

## AgriCUDa
Agriculture-oriented programming language and kernels (SbruiceS/AgriCUDa).
Endpoints under /api/v1/agricuda/. CPU device only. GPU/FPGA not implemented.
Explicit test double when binary absent — no fabricated numeric results.
