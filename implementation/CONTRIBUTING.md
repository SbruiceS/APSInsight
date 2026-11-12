# Contributing to APSInsight

## Development Workflow

1. Inspect existing code before making changes.
2. Follow the language responsibilities defined in ARCHITECTURE.md.
3. Add tests for all new functionality.
4. Update IMPLEMENTATION_STATUS.md and CHANGELOG.md.
5. Ensure scientific results include provenance.
6. Do not commit secrets.

## Code Standards

- TypeScript: strict mode.
- Ruby: follow Rails conventions + RSpec.
- C++: RAII, sanitizers in CI.
- Python: type hints, ruff + mypy.
- R: optional, must not break core platform if missing.

## Pull Requests

Describe the requirement, affected components, tests added, and status update.
