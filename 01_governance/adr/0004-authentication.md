# ADR-0004: Static hashed bearer tokens; OIDC through a proxy

- Status: Accepted
- Date: 2026-10-15

## Context

The master prompt asks for standards-based OIDC/OAuth2 "where appropriate". No identity provider is available in this environment, and implementing token validation without a real issuer to test against would produce untested security code.

## Decision

- `apsi-server` authenticates `Authorization: Bearer` tokens configured as `token:tenant:subject:role` entries. It keeps only SHA-256 hashes of them, and tokens must be at least 16 characters.
- The tenant always comes from the token, never from the request.
- For federation, deploy an OIDC-capable reverse proxy that maps identities to service tokens. Native OIDC (JWKS validation, audience and issuer checks, clock-skew limits) is future work and needs a test identity provider in CI.

## Consequences

- The authentication domain (password, passkeys, MFA, OIDC, SAML, token issuance, session renewal, account recovery) is recorded as **blocked by an external dependency** in the traceability matrix.
- Tokens are rotated by restarting the server with new values, and there is no session revocation list. Both are listed as residual risks in the [threat model](../threat-model.md).
