-- STATUS: SPECIFIED ONLY. Not executed by any test (ADR-0002). Target: PostGIS 3.4.
-- Geometries are stored in EPSG:4326 (03_domain_model/crs-policy.md); invalid
-- geometries are rejected rather than repaired silently.

BEGIN;

CREATE EXTENSION IF NOT EXISTS postgis;

CREATE TABLE apsi.record_geometries (
    tenant     text NOT NULL,
    id         text NOT NULL,
    role       text NOT NULL DEFAULT 'location' CHECK (role ~ '^[a-z0-9][a-z0-9_-]{0,63}$'),
    geom       geometry(Geometry, 4326) NOT NULL CHECK (ST_IsValid(geom)),
    source_crs text NOT NULL DEFAULT 'EPSG:4326' CHECK (source_crs ~ '^(EPSG:[0-9]{1,6}|OGC:CRS84)$'),
    PRIMARY KEY (tenant, id, role),
    FOREIGN KEY (tenant, id) REFERENCES apsi.records (tenant, id) ON DELETE CASCADE
);

CREATE INDEX record_geometries_gist ON apsi.record_geometries USING gist (geom);

ALTER TABLE apsi.record_geometries ENABLE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON apsi.record_geometries USING (tenant = current_setting('apsi.tenant', true));

COMMIT;
