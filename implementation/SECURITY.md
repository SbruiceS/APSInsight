# Security Policy

## Supported Versions

Currently in initial development. No production releases yet.

## Reporting a Vulnerability

Please report security issues privately to the ASPF security contact.

## Security Principles

- OpenID Connect / Keycloak for authentication.
- Organization- and project-scoped authorization.
- Never execute untrusted user code in the Rails process.
- Isolated analytical workers with resource limits.
- Parameterized queries, input validation, secret redaction.
- No credentials committed to the repository.
- SSRF, path traversal, and upload protections required for all ingestion endpoints.

See `docs/security/` for detailed controls.
