# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| main / implementation/polyglot-foundation | Active development |
| Tagged releases (none yet) | — |

No production releases yet. Security fixes are applied on the active branches.

## Reporting a Vulnerability

Please report security issues **privately**:

- Email the ASPF security contact, or
- Open a private security advisory on GitHub (Security → Advisories → Report a vulnerability).

Do **not** open a public issue for a vulnerability.

## Security Principles

- OpenID Connect / Keycloak for authentication (deferred; not yet enforced).
- Organization- and project-scoped authorization (Pundit policies present; membership checks to be wired).
- Never execute untrusted user code in the Rails process.
- Isolated analytical workers with resource limits.
- Parameterized queries, input validation, secret redaction.
- No credentials committed to the repository.
- SSRF, path traversal, and upload protections required for all ingestion endpoints.
- AgriQL and AgriCUDa adapters never fabricate results; they use real binaries or explicit test doubles.

## Repository Hardening

- **Dependabot** (`.github/dependabot.yml`) — weekly updates for Bundler, npm, pip, and GitHub Actions.
- **Security workflow** (`.github/workflows/security.yml`) — bundler-audit, npm audit, gitleaks secret scan.
- **`.gitignore`** excludes `.env`, keys, and build artifacts.
- **CI** runs on pull requests before merge.

## Recommended GitHub Settings (manual)

1. Enable **Dependabot alerts** and **secret scanning**.
2. Enable **push protection** for secrets.
3. Protect `main`: require pull request reviews and passing CI checks.
4. Restrict who can push to protected branches.
5. Enable **Code scanning** (CodeQL) when the codebase is larger.

See `docs/security/` for detailed controls.
