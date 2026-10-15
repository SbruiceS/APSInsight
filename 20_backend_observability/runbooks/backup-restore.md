# Runbook: backup and restore

A backup is **not valid until a restore from it has been tested** and the evidence recorded (cross-cutting criterion 14).

## What to back up

The data directory (`--data-dir`) holds exactly two files: `state.json` (snapshot) and `journal.ndjson` (commits since the snapshot). Tokens are configuration (`APSI_TOKENS`) and must be stored in your secret manager, not with the backup.

## Back up

1. Quiesce writes. Either stop the server (Ctrl+C or `docker compose stop apsinsight`; a graceful shutdown drains in-flight requests) or route traffic away from it.
2. Copy both files together, preserving the pair:
   ```sh
   tar -C /var/lib/apsinsight -czf apsinsight-$(date -u +%Y%m%dT%H%M%SZ).tgz state.json journal.ndjson
   sha256sum apsinsight-*.tgz > apsinsight.sha256
   ```
3. Restart the server. Store the archive and its checksum off-host, encrypted at rest.

Never copy only one of the two files. The journal is meaningful only over its snapshot.

## Restore (and the monthly restore drill)

1. Verify the archive: `sha256sum -c apsinsight.sha256`.
2. Extract it into an **empty** directory: `mkdir /tmp/restore && tar -C /tmp/restore -xzf apsinsight-<ts>.tgz`.
3. Verify integrity without starting the server. The open replays the journal and verifies the audit hash chain; it fails with exit code 9 on corruption:
   ```sh
   apsi --data-dir /tmp/restore --tenant <tenant> --role auditor audit verify
   apsi --data-dir /tmp/restore --tenant <tenant> dashboard
   ```
4. Compare record counts per domain with the pre-backup `dashboard` output saved at backup time.
5. For a real restore, stop the server, move the restored directory into place and start the server.
6. **Record the evidence:** date, archive name, checksum, `audit verify` output, the count comparison and the operator. Append it to your operations log. The release checklist ([acceptance criteria](../../02_product_management/acceptance-criteria.md)) requires a drill no older than 30 days.

## Automated coverage

`persistence_survives_restart_and_audit_chain_verifies` and `journal_compaction_torn_writes_and_metadata_survive_restart` test reopen, replay, torn writes and tamper detection. No automated test restores from an archive copy, so the drill above is manual.
