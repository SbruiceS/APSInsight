-- STATUS: SPECIFIED ONLY. This build does not use PostgreSQL (ADR-0002). No test
-- executes this file. It records the reviewed target schema for the future
-- PostgreSQL adapter: the same record envelope as crates/apsi-core/src/record.rs,
-- with tenant isolation enforced by the database as well as by the engine.
--
-- Target: PostgreSQL 16. Forward-only; rollback = restore the pre-migration backup.

BEGIN;

CREATE SCHEMA IF NOT EXISTS apsi;

CREATE TABLE apsi.records (
    tenant          text        NOT NULL CHECK (tenant ~ '^[a-z0-9][a-z0-9_-]{0,63}$'),
    id              text        NOT NULL CHECK (id ~ '^rec_[0-9a-f]{32}$'),
    domain          text        NOT NULL,
    entity          text        NOT NULL,
    schema_version  integer     NOT NULL CHECK (schema_version >= 1),
    version         bigint      NOT NULL CHECK (version >= 1),
    lifecycle       text        NOT NULL CHECK (lifecycle IN ('draft', 'active', 'archived')),
    name            text        NOT NULL CHECK (char_length(name) BETWEEN 1 AND 256),
    attributes      jsonb       NOT NULL DEFAULT '{}'::jsonb,
    tags            text[]      NOT NULL DEFAULT '{}',
    source_system   text,
    source_id       text,
    owner           text        NOT NULL,
    legal_hold      boolean     NOT NULL DEFAULT false,
    created_at      timestamptz NOT NULL,
    updated_at      timestamptz NOT NULL,
    created_by      text        NOT NULL,
    updated_by      text        NOT NULL,
    archived_at     timestamptz,
    PRIMARY KEY (tenant, id),
    CHECK ((source_system IS NULL) = (source_id IS NULL)),
    CHECK ((lifecycle = 'archived') = (archived_at IS NOT NULL))
);

-- Source identity is unique per tenant and entity kind (idempotent re-imports).
CREATE UNIQUE INDEX records_source_uq ON apsi.records (tenant, domain, entity, source_system, source_id) WHERE source_system IS NOT NULL;
CREATE INDEX records_kind_idx ON apsi.records (tenant, domain, entity, lifecycle);
CREATE INDEX records_attributes_gin ON apsi.records USING gin (attributes jsonb_path_ops);

CREATE TABLE apsi.record_versions (
    tenant     text   NOT NULL,
    id         text   NOT NULL,
    version    bigint NOT NULL,
    snapshot   jsonb  NOT NULL,
    PRIMARY KEY (tenant, id, version),
    FOREIGN KEY (tenant, id) REFERENCES apsi.records (tenant, id) ON DELETE CASCADE
);

CREATE TABLE apsi.provenance (
    tenant          text        NOT NULL,
    id              text        NOT NULL,
    seq             integer     NOT NULL,
    at              timestamptz NOT NULL,
    actor           text        NOT NULL,
    actor_kind      text        NOT NULL CHECK (actor_kind IN ('user', 'service')),
    action          text        NOT NULL,
    source_system   text,
    source_id       text,
    transformation  text,
    inputs          text[]      NOT NULL DEFAULT '{}',
    PRIMARY KEY (tenant, id, seq),
    FOREIGN KEY (tenant, id) REFERENCES apsi.records (tenant, id) ON DELETE CASCADE
);

-- Hash-chained audit log; append-only for the application role.
CREATE TABLE apsi.audit_events (
    seq        bigint      PRIMARY KEY,
    at         timestamptz NOT NULL,
    tenant     text        NOT NULL,
    actor      text        NOT NULL,
    action     text        NOT NULL,
    target     text        NOT NULL,
    outcome    text        NOT NULL,
    detail     text,
    prev_hash  char(64)    NOT NULL,
    hash       char(64)    NOT NULL UNIQUE
);

-- Row-level security: the adapter sets `apsi.tenant` per transaction from the
-- authenticated principal; a missing setting matches no rows.
ALTER TABLE apsi.records ENABLE ROW LEVEL SECURITY;
ALTER TABLE apsi.record_versions ENABLE ROW LEVEL SECURITY;
ALTER TABLE apsi.provenance ENABLE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON apsi.records USING (tenant = current_setting('apsi.tenant', true));
CREATE POLICY tenant_isolation ON apsi.record_versions USING (tenant = current_setting('apsi.tenant', true));
CREATE POLICY tenant_isolation ON apsi.provenance USING (tenant = current_setting('apsi.tenant', true));

COMMIT;
