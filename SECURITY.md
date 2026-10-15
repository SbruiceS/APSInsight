# Security policy

## Reporting a vulnerability

Please report vulnerabilities privately through the repository's **Security → Report a vulnerability** form (GitHub private vulnerability reporting). Do not open a public issue.

Include the affected version or commit, the steps to reproduce, the impact as you understand it, and whether the issue is already public. We aim to acknowledge reports within 3 working days and to agree a disclosure date with the reporter. There is no bug bounty.

## Supported versions

APSInsight is pre-1.0. Only the latest commit on `main` receives security fixes.

## Dependency update policy

- Rust and npm dependencies are pinned by `Cargo.lock` and `web/package-lock.json`.
- CI runs `cargo deny check` (advisories, licences, bans, sources; configuration in [deny.toml](deny.toml)), `npm audit --audit-level=high`, a secret scan (gitleaks) and `node tools/sbom.mjs`, which writes the CycloneDX SBOMs and fails on licences outside the allow-list.
- Advisories rated high or critical are patched, or recorded with a justification in `deny.toml`, within 7 days. Others are handled in the next regular update.
- Dependencies are reviewed monthly. A major-version upgrade gets its own change and changelog entry.

## Where the details are

- [Threat model](01_governance/threat-model.md): assets, data classification, trust boundaries, threats, controls and the tests that evidence them.
- [docs/SECURITY.md](docs/SECURITY.md): the security review of the implemented controls.
- [Incident response runbook](20_backend_observability/runbooks/incident-response.md).
