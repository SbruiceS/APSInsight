# CI/CD Pipeline for APSInsight

## Current CI (GitHub Actions)

Workflow: `.github/workflows/ci.yml`

| Job | What it does |
|-----|----------------|
| **Rails** | Ruby 3.3, bundle install, model specs |
| **Frontend** | Node 20, npm install, typecheck, build |
| **Python** | Python 3.11, install package, pytest |
| **C++** | CMake configure, build, ctest |
| **Docker** | `docker compose config` validation |

Triggers on push to `main`, `master`, and `implementation/polyglot-foundation`, plus all pull requests.

## Recommended CD stages

### 1. Build & test (already in CI)
- All language jobs must pass before any deploy.

### 2. Container images
- Build multi-stage Docker images:
  - `apsinsight-rails` (backend)
  - `apsinsight-frontend` (static build served by nginx or Rails)
- Push to GitHub Container Registry (`ghcr.io`) on tags or main.

### 3. Deploy (staging)
- On push to `main` (or a release branch):
  - Deploy to a staging environment (Docker Compose on a VM, or Kubernetes).
  - Run database migrations (`rails db:migrate`).
  - Run a smoke test against `/api/v1/health/live`.

### 4. Deploy (production)
- On version tags (`v*`) or manual approval:
  - Deploy to production.
  - Optional blue/green or rolling update.
  - Post-deploy health check.

### 5. Optional extras
- **Dependabot** or Renovate for Gemfile, package.json, requirements.
- **Security scanning**: `bundle audit`, `npm audit`, `trivy` on images.
- **Coverage**: SimpleCov (Rails), coverage.py (Python).
- **Path filters**: only run Rails job when `backend/rails/**` changes, etc.

## Minimal next steps

1. Add a PostgreSQL service container to the Rails job so `db:migrate` and full specs can run.
2. Add a `deploy-staging.yml` workflow that builds and pushes images after CI succeeds.
3. Protect the `main` branch (require CI to pass before merge).

The current `ci.yml` is ready to use and can be extended with the CD stages above.
