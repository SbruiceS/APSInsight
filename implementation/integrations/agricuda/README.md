# AgriCUDa Kernel Integration

## Source

Private repository: `SbruiceS/AgriCUDa` (Apache-2.0).

Agriculture-oriented programming language. Programs are type-checked, compiled to bytecode, and executed by a stack machine. A reference tree-walk interpreter must agree.

## Real capabilities

| Feature | State |
|---------|--------|
| `agricuda check` | Implemented |
| `agricuda run` | Implemented (bytecode + stack machine) |
| `agricuda ir` | Implemented |
| `kernel fn` + `launch <<<grid, block>>>` | Implemented on CPU device |
| `parallel_for` | Host loop |
| GPU backend | **Not implemented** |
| FPGA backend | **Not implemented** |
| Native object code / `build` | **Not implemented** |

Example kernel (from AgriCUDa README):

```
kernel fn saxpy(a: f64, x: Array<f64>, y: Array<f64>, z: Array<f64>) {
  let i = block_idx * block_dim + thread_idx;
  if i < x.length { z[i] = a * x[i] + y[i]; }
}
```

SAXPY example sum is 120 (verified by AgriCUDa tests).

## APSInsight integration

- Service: `AgricudaAdapter` (`app/services/agricuda_adapter.rb`)
- Endpoints:
  - `GET /api/v1/agricuda/capabilities`
  - `POST /api/v1/agricuda/run` `{ "source": "..." }`
  - `POST /api/v1/agricuda/check` `{ "source": "..." }`
- Python wrapper: `python/src/apsinsight/integrations/agricuda_client.py`
- When the `agricuda` binary is absent the adapter returns an explicit test-double status and **does not fabricate numeric results**.

Set `AGRICUDA_BIN` if the binary is not on `PATH`.
