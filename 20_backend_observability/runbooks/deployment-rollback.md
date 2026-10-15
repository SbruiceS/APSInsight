# Runbook: deployment and rollback

## Deploy

1. CI `verify` and the security jobs are green for the commit.
2. Build the image: `docker build -t apsinsight:<git-sha> .` (see [Dockerfile](../../Dockerfile)). It contains no secrets.
3. **Back up** the data directory ([backup-restore](backup-restore.md)) and keep the archive until the new release has run for 7 days.
4. Deploy with `docker compose up -d` (tokens come from the environment or a secrets file; see [compose.yaml](../../compose.yaml)).
5. Smoke test: `GET /health` returns `ok` and the expected version; `GET /api/v1/audit/verify` returns `valid: true`; one read in a known tenant succeeds.

## Roll back

Rolling back an application is safe only if the on-disk format is compatible ([compatibility matrix](../../01_governance/release/COMPATIBILITY.md)).

1. Stop the new container.
2. If the new release wrote data in a newer format, which the release notes must say, restore the pre-deployment backup. Otherwise keep the current data directory.
3. Start the previous image tag against the data directory.
4. Repeat the smoke test.
5. Schema migrations applied through the API are rolled back separately with `POST /api/v1/migrations/{id}/rollback`. Records edited after the migration are reported as conflicts and left unchanged.

## Not automated

There is no automated rollout or rollback, no image signing and no canary. These are recorded gaps in the deployment-and-infrastructure domain ([STATUS](../../docs/STATUS.md)).
