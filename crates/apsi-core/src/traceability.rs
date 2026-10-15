//! Feature traceability: maps every register feature (domain x capability) to the
//! code, API routes, CLI commands, UI views and tests that implement it.
//!
//! Because each capability is implemented once and applied to every entity kind,
//! the evidence for a feature is the capability's evidence instantiated for the
//! feature's domain. The test `every_referenced_test_exists` keeps this table honest.

use crate::catalog::{Capability, Domain, catalog};
use serde::Serialize;

pub struct Evidence {
    pub capability: &'static str,
    /// Status of the generic capability before any domain scope is applied.
    pub status: Status,
    /// What is missing, if anything. Empty only for fully implemented and tested capabilities.
    pub gap: &'static str,
    pub engine: &'static [&'static str],
    /// `{domain}` is substituted with the feature's domain slug.
    pub api: &'static [&'static str],
    pub cli: &'static [&'static str],
    pub ui: &'static str,
    /// `crate::path::test_name` entries.
    pub tests: &'static [&'static str],
    pub docs: &'static str,
}

macro_rules! k {
    ($m:literal, $p:literal) => {
        concat!($m, " /api/v1/domains/{domain}/entities/{entity}", $p)
    };
}

pub const EVIDENCE: [Evidence; 31] = [
    Evidence {
        capability: "registration-and-identity",
        status: Status::ImplementedAndTested,
        gap: "",
        engine: &["Engine::create", "ids::new_record_id", "record::SourceRef (unique per entity kind)"],
        api: &[k!("POST", "/records"), "GET /api/v1/records/{id}"],
        cli: &["apsi records create", "apsi records get"],
        ui: "Domain workspace > Records > New record",
        tests: &[
            "apsi-core::engine::registration_assigns_stable_ids_and_preserves_source_identity",
            "apsi-core::ids::tests::record_ids_are_unique_and_valid",
            "apsi-server::api::api_create_get_and_envelopes",
        ],
        docs: "docs/ARCHITECTURE.md#records",
    },
    Evidence {
        capability: "schema-contract",
        status: Status::ImplementedAndTested,
        gap: "",
        engine: &["schema::EntitySchema", "EntitySchema::compatibility", "Engine::publish_schema"],
        api: &[k!("GET", "/schemas"), k!("POST", "/schemas?allow_breaking=")],
        cli: &["apsi schema show", "apsi schema publish"],
        ui: "Domain workspace > Schema",
        tests: &[
            "apsi-core::schema::tests::compatibility_rules",
            "apsi-core::schema::tests::malformed_schemas_are_rejected",
            "apsi-core::engine::schema_publication_enforces_compatibility",
            "apsi-core::engine::record_envelope_matches_published_json_schema",
            "apsi-server::api::api_schema_rules_and_validation",
        ],
        docs: "docs/API.md#schemas",
    },
    Evidence {
        capability: "metadata-model",
        status: Status::ImplementedAndTested,
        gap: "",
        engine: &["record::Record (owner, timestamps, schema_version, tags, source)", "schema::FieldDef (unit, ranges)", "record::ProvenanceEvent"],
        api: &["GET /api/v1/records/{id}"],
        cli: &["apsi records get"],
        ui: "Record detail",
        tests: &[
            "apsi-core::schema::tests::default_schema_accepts_valid_and_rejects_malformed",
            "apsi-core::engine::registration_assigns_stable_ids_and_preserves_source_identity",
        ],
        docs: "docs/ARCHITECTURE.md#metadata",
    },
    Evidence {
        capability: "validation-rules",
        status: Status::ImplementedAndTested,
        gap: "",
        engine: &["validation::validate", "validation::ValidationRule (severity)", "Engine::set_rules", "Engine::validate_preview"],
        api: &[k!("GET", "/rules"), k!("PUT", "/rules"), k!("POST", "/validate")],
        cli: &["apsi rules show", "apsi rules set", "apsi records validate"],
        ui: "Domain workspace > Rules; validation report panel",
        tests: &[
            "apsi-core::validation::tests::severities_drive_validity",
            "apsi-core::engine::configurable_rules_with_severities",
            "apsi-server::api::api_schema_rules_and_validation",
        ],
        docs: "docs/API.md#validation",
    },
    Evidence {
        capability: "crud-and-lifecycle",
        status: Status::ImplementedAndTested,
        gap: "",
        engine: &["Engine::create", "Engine::get", "Engine::update", "Engine::transition", "Engine::delete", "record::Lifecycle::can_transition"],
        api: &[k!("POST", "/records"), "PATCH /api/v1/records/{id}", "POST /api/v1/records/{id}/lifecycle", "DELETE /api/v1/records/{id}"],
        cli: &["apsi records create|get|update|archive|restore|delete"],
        ui: "Records table actions; record detail",
        tests: &[
            "apsi-core::engine::crud_lifecycle_and_archive_constraints",
            "apsi-core::record::tests::lifecycle_transitions",
            "apsi-server::api::api_lifecycle_concurrency_and_history",
        ],
        docs: "docs/ARCHITECTURE.md#lifecycle",
    },
    Evidence {
        capability: "search-and-filtering",
        status: Status::ImplementedAndTested,
        gap: "",
        engine: &["Engine::search (partition + exact-match index)", "search::Query", "search::Filter"],
        api: &[k!("GET", "/records?q=&tags=&facets=&offset=&limit="), "POST /api/v1/domains/{domain}/search"],
        cli: &["apsi records search"],
        ui: "Records filter bar, facets, pagination",
        tests: &[
            "apsi-core::engine::search_supports_exact_ranges_facets_and_pagination",
            "apsi-core::search::tests::query_bounds",
            "apsi-server::api::api_search_bulk_import_export",
        ],
        docs: "docs/API.md#search",
    },
    Evidence {
        capability: "bulk-operations",
        status: Status::ImplementedAndTested,
        gap: "",
        engine: &["Engine::bulk (dry_run, upsert, idempotency_key)", "bulk::BulkReport"],
        api: &[k!("POST", "/bulk")],
        cli: &["apsi records bulk"],
        ui: "Import dialog (preview then apply)",
        tests: &[
            "apsi-core::engine::bulk_import_reports_partial_failures_dry_runs_and_is_idempotent",
            "apsi-core::engine::bulk_upsert_by_source_identity",
            "apsi-server::api::api_search_bulk_import_export",
        ],
        docs: "docs/API.md#bulk",
    },
    Evidence {
        capability: "versioning",
        status: Status::ImplementedAndTested,
        gap: "",
        engine: &["Engine::history", "Engine::diff", "Engine::rollback", "record::diff"],
        api: &["GET /api/v1/records/{id}/history", "GET /api/v1/records/{id}/diff?from=&to=", "POST /api/v1/records/{id}/rollback"],
        cli: &["apsi records history", "apsi records diff", "apsi records rollback"],
        ui: "Record detail > History (diff, rollback)",
        tests: &["apsi-core::engine::versioning_history_diff_and_rollback", "apsi-server::api::api_lifecycle_concurrency_and_history"],
        docs: "docs/ARCHITECTURE.md#versioning",
    },
    Evidence {
        capability: "access-control",
        status: Status::ImplementedAndTested,
        gap: "",
        engine: &["auth::Principal::authorize (tenant first, then role)", "auth::TokenRegistry (hashed tokens)", "Engine::tx / Engine::read (denials audited)"],
        api: &["Authorization: Bearer <token> on every /api/v1 route"],
        cli: &["--tenant --actor --role (local operator principal)"],
        ui: "Sign-in token; permission-denied states",
        tests: &[
            "apsi-core::auth::tests::role_matrix_allowed_and_denied",
            "apsi-core::auth::tests::cross_tenant_access_is_denied_even_for_admin",
            "apsi-core::engine::access_control_allows_and_denies_across_roles_and_tenants",
            "apsi-server::api::api_authentication_and_tenant_isolation",
        ],
        docs: "docs/SECURITY.md#access-control",
    },
    Evidence {
        capability: "provenance",
        status: Status::ImplementedAndTested,
        gap: "",
        engine: &["record::ProvenanceEvent (actor, actor_kind, source, transformation, inputs)", "audit::AuditLog"],
        api: &["GET /api/v1/records/{id}", "GET /api/v1/records/{id}/history", "GET /api/v1/audit"],
        cli: &["apsi records get", "apsi audit list"],
        ui: "Record detail > Provenance",
        tests: &["apsi-core::engine::versioning_history_diff_and_rollback", "apsi-core::engine::registration_assigns_stable_ids_and_preserves_source_identity"],
        docs: "docs/ARCHITECTURE.md#provenance",
    },
    Evidence {
        capability: "quality-indicators",
        status: Status::ImplementedAndTested,
        gap: "",
        engine: &["quality::aggregate", "quality::FORMULAS", "quality::MISSING_DATA", "quality::LIMITATIONS"],
        api: &[k!("GET", "/quality")],
        cli: &["apsi quality"],
        ui: "Domain workspace > Quality",
        tests: &["apsi-core::engine::quality_indicators_with_documented_formulas_and_empty_behavior"],
        docs: "docs/ARCHITECTURE.md#quality",
    },
    Evidence {
        capability: "observability",
        status: Status::ImplementedNotFullyTested,
        gap: "Structured JSON logs, request ids and Prometheus metrics are implemented; OpenTelemetry traces are not exported.",
        engine: &["metrics::Metrics (Prometheus)", "metrics::redact", "tracing JSON logs with request ids"],
        api: &["GET /metrics (audit permission)", "x-request-id header"],
        cli: &["apsi --log-json"],
        ui: "Activity feed on dashboards",
        tests: &[
            "apsi-core::engine::observability_counts_operations_without_payloads",
            "apsi-core::metrics::tests::redaction",
            "apsi-server::api::api_metrics_security_headers_and_limits",
        ],
        docs: "docs/OPERATIONS.md#observability",
    },
    Evidence {
        capability: "failure-handling",
        status: Status::ImplementedAndTested,
        gap: "",
        engine: &["error::ApsiError (code, retryable, http_status, exit_code)", "transactional rollback in Engine::tx"],
        api: &["error envelope { code, message, retryable, details }"],
        cli: &["stable exit codes 0..10"],
        ui: "Error states with retry for retryable errors",
        tests: &[
            "apsi-core::error::tests::retryable_classification_is_explicit",
            "apsi-core::engine::failed_transactions_roll_back_completely",
            "apsi-server::api::api_errors_use_typed_envelopes",
        ],
        docs: "docs/API.md#errors",
    },
    Evidence {
        capability: "concurrency-control",
        status: Status::ImplementedAndTested,
        gap: "",
        engine: &["Engine::tx (single-writer transaction, snapshot rollback)", "expected_version optimistic concurrency"],
        api: &["PATCH /api/v1/records/{id} (If-Match / expected_version)", "ETag on GET"],
        cli: &["apsi records update --expected-version"],
        ui: "Conflict message with reload",
        tests: &[
            "apsi-core::engine::optimistic_concurrency_rejects_stale_writes",
            "apsi-core::engine::concurrent_writers_never_lose_updates",
            "apsi-server::api::api_lifecycle_concurrency_and_history",
        ],
        docs: "docs/ARCHITECTURE.md#concurrency",
    },
    Evidence {
        capability: "retention-policy",
        status: Status::ImplementedAndTested,
        gap: "",
        engine: &["retention::RetentionPolicy", "Engine::apply_retention", "Engine::set_legal_hold", "Engine::delete"],
        api: &[k!("GET", "/retention"), k!("PUT", "/retention"), "POST /api/v1/retention/apply?dry_run=", "PUT /api/v1/records/{id}/legal-hold"],
        cli: &["apsi retention set", "apsi retention apply", "apsi records hold"],
        ui: "Domain workspace > Retention",
        tests: &["apsi-core::engine::retention_archives_purges_and_respects_legal_hold"],
        docs: "docs/ARCHITECTURE.md#retention",
    },
    Evidence {
        capability: "export-contract",
        status: Status::ImplementedAndTested,
        gap: "",
        engine: &["Engine::export", "exchange::render (json, ndjson, csv, geojson)", "exchange::ExportHeader"],
        api: &[k!("GET", "/export?format=")],
        cli: &["apsi records export"],
        ui: "Export menu",
        tests: &["apsi-core::engine::export_import_roundtrip_is_lossless_in_csv_and_json", "apsi-server::api::api_search_bulk_import_export"],
        docs: "docs/API.md#export",
    },
    Evidence {
        capability: "import-contract",
        status: Status::ImplementedAndTested,
        gap: "",
        engine: &["Engine::import", "exchange::csv_to_records", "exchange::json_to_records", "exchange::FieldMapping"],
        api: &[k!("POST", "/import?format=&dry_run=&upsert="), k!("POST", "/bulk (mapping)")],
        cli: &["apsi records import"],
        ui: "Import dialog with preview and row errors",
        tests: &[
            "apsi-core::engine::import_mapping_preview_and_row_errors",
            "apsi-core::exchange::tests::csv_to_records_maps_columns",
            "apsi-server::api::api_search_bulk_import_export",
        ],
        docs: "docs/API.md#import",
    },
    Evidence {
        capability: "api-endpoint",
        status: Status::ImplementedAndTested,
        gap: "",
        engine: &["apsi-server::router", "apsi-server::api::routes"],
        api: &["/api/v1/* (versioned, bearer auth, 16 MiB limit, rate limit, {data|error, meta} envelopes)"],
        cli: &["apsi-server --bind --data-dir --tokens"],
        ui: "web/src/app/core/api.service.ts",
        tests: &[
            "apsi-server::api::api_create_get_and_envelopes",
            "apsi-server::api::api_errors_use_typed_envelopes",
            "apsi-server::api::api_metrics_security_headers_and_limits",
            "apsi-server::auth::tests::limiter_blocks_after_budget",
            "apsi-server::api::openapi_documents_every_route",
        ],
        docs: "docs/API.md",
    },
    Evidence {
        capability: "cli-support",
        status: Status::ImplementedAndTested,
        gap: "",
        engine: &["apsi-cli (JSON output, stable exit codes, no prompts)"],
        api: &[],
        cli: &["apsi <group> <command> --data-dir --tenant --actor --role"],
        ui: "-",
        tests: &["apsi-cli::cli::cli_json_output_and_exit_codes", "apsi-cli::cli::cli_import_export_and_features"],
        docs: "docs/CLI.md",
    },
    Evidence {
        capability: "ui-workflow",
        status: Status::ImplementedNotFullyTested,
        gap: "Covered by AOT template type-checking, static UI checks and unit tests; there are no browser end-to-end tests.",
        engine: &["web/ (Angular)"],
        api: &["consumes /api/v1"],
        cli: &[],
        ui: "Records list/detail with empty, loading, error and permission-denied states",
        tests: &["web::build (AOT template type-checking)", "web::scripts/check-ui.mjs"],
        docs: "docs/UI.md",
    },
    Evidence {
        capability: "dashboard-visibility",
        status: Status::ImplementedAndTested,
        gap: "",
        engine: &["Engine::dashboard", "Engine::overview", "dashboard::EntitySummary"],
        api: &["GET /api/v1/overview", "GET /api/v1/domains/{domain}/dashboard"],
        cli: &["apsi dashboard"],
        ui: "Overview and domain dashboards with drill-down links",
        tests: &["apsi-core::engine::dashboard_and_overview_summaries", "apsi-server::api::api_dashboard_notifications_and_workflows"],
        docs: "docs/UI.md#dashboards",
    },
    Evidence {
        capability: "notification-rules",
        status: Status::ImplementedAndTested,
        gap: "",
        engine: &["notify::NotificationRule", "dedup window", "outbox", "notification audit events"],
        api: &["GET/POST /api/v1/notification-rules", "DELETE /api/v1/notification-rules/{id}", "GET /api/v1/notifications"],
        cli: &["apsi notify add|list|outbox"],
        ui: "Notifications page",
        tests: &["apsi-core::engine::notification_rules_deduplicate_and_audit", "apsi-server::api::api_dashboard_notifications_and_workflows"],
        docs: "docs/API.md#notifications",
    },
    Evidence {
        capability: "workflow-integration",
        status: Status::ImplementedAndTested,
        gap: "",
        engine: &[
            "workflow::WorkflowDefinition::check (typed contracts)",
            "Engine::run_workflow (definition hash + data fingerprint)",
            "apsi-science::ScienceRegistry",
        ],
        api: &["GET/POST /api/v1/workflows", "POST /api/v1/workflows/{id}/runs", "GET /api/v1/compute", "POST /api/v1/compute/{function}"],
        cli: &["apsi workflow register|run", "apsi compute"],
        ui: "Workflows page",
        tests: &[
            "apsi-core::engine::workflows_are_type_checked_and_reproducible",
            "apsi-science::registry::tests::calls_dispatch",
            "apsi-server::api::api_dashboard_notifications_and_workflows",
        ],
        docs: "docs/ARCHITECTURE.md#workflows",
    },
    Evidence {
        capability: "test-coverage",
        status: Status::ImplementedAndTested,
        gap: "",
        engine: &["unit, integration, contract (HTTP) and property-based (proptest) suites"],
        api: &[],
        cli: &["cargo test --workspace"],
        ui: "-",
        tests: &[
            "apsi-core::ids::tests::validation_never_panics",
            "apsi-core::exchange::tests::csv_cells_roundtrip",
            "apsi-core::validation::tests::validation_is_total_on_arbitrary_input",
            "apsi-science::stats::tests::bh_adjusted_are_valid_probabilities",
            "apsi-core::engine::unknown_entity_kinds_and_malformed_input_are_rejected",
        ],
        docs: "docs/TESTING.md",
    },
    Evidence {
        capability: "security-review",
        status: Status::ImplementedAndTested,
        gap: "",
        engine: &[
            "ids::validate_slug / validate_record_id (injection resistance)",
            "hashed tokens",
            "rate limiting",
            "body limits",
            "redaction",
            "tamper-evident audit chain",
        ],
        api: &["security headers on /api", "401/403/413/429 envelopes"],
        cli: &[],
        ui: "Token kept in session storage only",
        tests: &[
            "apsi-core::ids::tests::slugs",
            "apsi-core::audit::tests::chain_detects_tampering",
            "apsi-core::engine::persistence_survives_restart_and_audit_chain_verifies",
            "apsi-server::api::api_authentication_and_tenant_isolation",
            "apsi-server::api::api_metrics_security_headers_and_limits",
        ],
        docs: "docs/SECURITY.md",
    },
    Evidence {
        capability: "performance-target",
        status: Status::ImplementedNotFullyTested,
        gap: "Measured on one developer machine with a synthetic workload; no per-domain workloads, latency percentiles or CI regression gate.",
        engine: &["apsi bench (documented workload, measured numbers only)"],
        api: &[],
        cli: &["apsi bench --records N"],
        ui: "-",
        tests: &["apsi-cli::cli::cli_bench_reports_measured_numbers"],
        docs: "docs/PERFORMANCE.md",
    },
    Evidence {
        capability: "documentation",
        status: Status::ImplementedAndTested,
        gap: "",
        engine: &["docs::domain_markdown (generated per domain)"],
        api: &["GET /api/v1/features/{id}", "GET /api/v1/catalog"],
        cli: &["apsi docs generate"],
        ui: "Feature register browser",
        tests: &["apsi-cli::cli::generated_docs_are_up_to_date"],
        docs: "docs/domains/{domain}.md",
    },
    Evidence {
        capability: "interoperability",
        status: Status::ImplementedAndTested,
        gap: "",
        engine: &[
            "exchange (CSV RFC 4180, NDJSON, GeoJSON RFC 7946)",
            "apsi-science::geo (EPSG:4326 <-> EPSG:3857)",
            "apsi-science::agronomy::convert (NIST factors)",
        ],
        api: &[k!("GET", "/export?format=geojson")],
        cli: &["apsi records export --format geojson", "apsi compute agro.convert"],
        ui: "Export menu",
        tests: &[
            "apsi-core::engine::export_import_roundtrip_is_lossless_in_csv_and_json",
            "apsi-science::geo::tests::web_mercator_roundtrip",
            "apsi-science::agronomy::tests::conversions_and_moisture",
            "apsi-science::units::tests::invalid_dimensional_operations_are_rejected",
            "apsi-science::units::tests::conversions_round_trip",
        ],
        docs: "docs/INTEROPERABILITY.md",
    },
    Evidence {
        capability: "accessibility-and-localization",
        status: Status::ImplementedNotFullyTested,
        gap: "Static checks only (labels, aria-live, catalogue completeness); no automated axe run, screen-reader test or recorded manual audit.",
        engine: &["web/src/app/core/i18n.ts (en, hi catalogues)"],
        api: &[],
        cli: &[],
        ui: "Keyboard navigable views, aria-live status region, skip link, translated labels",
        tests: &["web::scripts/check-ui.mjs (catalogue completeness, aria-live, labelled controls)"],
        docs: "docs/UI.md#accessibility",
    },
    Evidence {
        capability: "migration-support",
        status: Status::ImplementedAndTested,
        gap: "",
        engine: &["Engine::migrate (dry run, all-or-nothing)", "Engine::rollback_migration", "migration::MigrationOp"],
        api: &[k!("POST", "/migrations?dry_run="), "POST /api/v1/migrations/{id}/rollback"],
        cli: &["apsi migrate", "apsi migrate rollback"],
        ui: "Domain workspace > Schema > Migrate",
        tests: &[
            "apsi-core::engine::migrations_preview_apply_and_roll_back",
            "apsi-core::engine::failed_migrations_change_nothing",
            "apsi-core::migration::tests::ops_apply_in_order",
        ],
        docs: "docs/ARCHITECTURE.md#migrations",
    },
    Evidence {
        capability: "acceptance-evidence",
        status: Status::ImplementedAndTested,
        gap: "",
        engine: &["traceability (this matrix)"],
        api: &["GET /api/v1/traceability"],
        cli: &["apsi features trace"],
        ui: "Feature register browser > evidence",
        tests: &[
            "apsi-core::engine::acceptance_every_entity_kind_supports_core_capabilities",
            "apsi-core::traceability::tests::every_feature_has_evidence",
            "apsi-server::api::api_acceptance_sweep_all_domains",
        ],
        docs: "docs/traceability.csv",
    },
];

/// The four honest statuses required by the master build prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    ImplementedAndTested,
    ImplementedNotFullyTested,
    SpecifiedOnly,
    BlockedExternal,
}

impl Status {
    pub const ALL: [Status; 4] = [Self::ImplementedAndTested, Self::ImplementedNotFullyTested, Self::SpecifiedOnly, Self::BlockedExternal];

    pub fn label(self) -> &'static str {
        match self {
            Self::ImplementedAndTested => "implemented and tested",
            Self::ImplementedNotFullyTested => "implemented but not fully tested",
            Self::SpecifiedOnly => "specified only",
            Self::BlockedExternal => "blocked by an external dependency",
        }
    }
}

/// A domain whose substance needs something this build does not have. The scope
/// caps the status of every capability except [`UNCAPPED`] ones, which describe
/// the domain's records and contract rather than its runtime behaviour.
pub struct DomainScope {
    pub domain: &'static str,
    pub status: Status,
    pub note: &'static str,
}

/// Capabilities whose evidence (schemas, metadata, generated documentation) holds
/// regardless of whether the domain's runtime function exists.
pub const UNCAPPED: [&str; 3] = ["schema-contract", "metadata-model", "documentation"];

pub const DOMAIN_SCOPES: &[DomainScope] = &[
    DomainScope {
        domain: "authentication",
        status: Status::BlockedExternal,
        note: "Password, passkey, MFA, OIDC and SAML flows need an identity provider; this build authenticates static bearer tokens only (ADR-0004).",
    },
    DomainScope { domain: "realtime-communications", status: Status::SpecifiedOnly, note: "No WebSocket or server-sent-events endpoint exists; clients poll." },
    DomainScope {
        domain: "relational-data-services",
        status: Status::BlockedExternal,
        note: "PostgreSQL is not deployed; storage is the embedded journal (ADR-0002). The SQL migrations in 32_storage_postgres are specified and not executed.",
    },
    DomainScope {
        domain: "object-and-file-storage",
        status: Status::BlockedExternal,
        note: "No object store is integrated; upload sessions, multipart uploads and signed URLs do not exist.",
    },
    DomainScope { domain: "columnar-and-analytical-storage", status: Status::BlockedExternal, note: "Arrow, Parquet and DuckDB are not integrated." },
    DomainScope {
        domain: "raster-and-satellite-imagery",
        status: Status::BlockedExternal,
        note: "No raster I/O (GeoTIFF/COG) is integrated; only per-pixel index formulas exist in apsi-science.",
    },
    DomainScope {
        domain: "satellite-and-agency-integrations",
        status: Status::BlockedExternal,
        note: "No provider connectors; provider terms and data rights have not been verified.",
    },
    DomainScope {
        domain: "external-data-connectors",
        status: Status::BlockedExternal,
        note: "No connector runtime or credential store; only the generic import contract exists.",
    },
    DomainScope {
        domain: "natural-language-assistance",
        status: Status::BlockedExternal,
        note: "Needs KutajKutambi AI or another vetted model, neither of which is available.",
    },
    DomainScope { domain: "computer-vision", status: Status::BlockedExternal, note: "No image store or inference runtime is integrated." },
    DomainScope {
        domain: "machine-learning-lifecycle",
        status: Status::SpecifiedOnly,
        note: "No training or evaluation runtime; only metadata records of runs and models.",
    },
    DomainScope {
        domain: "background-job-execution",
        status: Status::SpecifiedOnly,
        note: "apsi-jobs provides a tested, deterministic tenant-safe lifecycle core (leases, cancellation, retry and expired-lease recovery), but has no durable adapter, API route or deployed worker; workflows still run synchronously.",
    },
    DomainScope {
        domain: "scheduling-and-resource-management",
        status: Status::SpecifiedOnly,
        note: "The lifecycle core supports scheduled availability and retry backoff, but there is no scheduler, quota or admission-control runtime.",
    },
    DomainScope { domain: "plugin-and-extension-system", status: Status::SpecifiedOnly, note: "No plugin loader or sandbox." },
    DomainScope {
        domain: "multi-tenant-billing-and-quotas",
        status: Status::SpecifiedOnly,
        note: "Only per-token rate limits exist; no metering or quota enforcement.",
    },
    DomainScope { domain: "edge-and-offline-operations", status: Status::SpecifiedOnly, note: "No offline capture, sync manifests or conflict resolution." },
    DomainScope { domain: "gis-map-workspace", status: Status::SpecifiedOnly, note: "There is no map view in the UI." },
    DomainScope {
        domain: "data-ingestion",
        status: Status::ImplementedNotFullyTested,
        note: "CSV, JSON and NDJSON only; no Excel, Parquet, database or streaming ingestion.",
    },
    DomainScope {
        domain: "time-series-data",
        status: Status::ImplementedNotFullyTested,
        note: "Gap detection, late arrivals and downsampling exist as compute functions; no retention tiers or temporal joins.",
    },
    DomainScope {
        domain: "geospatial-foundations",
        status: Status::ImplementedNotFullyTested,
        note: "WGS 84/Web Mercator, distance, area and containment only; no spatial index, buffering, overlays or spatial joins.",
    },
    DomainScope {
        domain: "iot-event-processing",
        status: Status::ImplementedNotFullyTested,
        note: "No stream windows, replay or dead-letter handling; the APSNetwork telemetry contract is defined.",
    },
    DomainScope {
        domain: "genomics-and-sequence-metadata",
        status: Status::ImplementedNotFullyTested,
        note: "Metadata records only; no sequence or variant processing.",
    },
    DomainScope {
        domain: "transcriptomics-and-gene-expression",
        status: Status::ImplementedNotFullyTested,
        note: "Metadata records only; no normalisation or differential-expression analysis.",
    },
    DomainScope { domain: "proteomics-and-metabolomics", status: Status::ImplementedNotFullyTested, note: "Metadata records only; no abundance processing." },
    DomainScope {
        domain: "biological-pathways-and-ontology",
        status: Status::ImplementedNotFullyTested,
        note: "Metadata records only; no ontology import or enrichment analysis.",
    },
    DomainScope {
        domain: "causal-and-comparative-analysis",
        status: Status::ImplementedNotFullyTested,
        note: "Welch t-tests and multiple-testing correction exist; no causal-effect estimators or sensitivity analysis.",
    },
    DomainScope {
        domain: "forecasting-and-prediction",
        status: Status::ImplementedNotFullyTested,
        note: "Forecast error metrics exist; no forecasting models or backtesting.",
    },
    DomainScope { domain: "anomaly-and-fault-detection", status: Status::ImplementedNotFullyTested, note: "No anomaly-scoring models." },
    DomainScope { domain: "optimization-and-simulation", status: Status::ImplementedNotFullyTested, note: "No solvers or simulation runtime." },
    DomainScope { domain: "knowledge-graph-and-semantics", status: Status::ImplementedNotFullyTested, note: "No graph store or traversal." },
    DomainScope {
        domain: "search-and-discovery",
        status: Status::ImplementedNotFullyTested,
        note: "Exact, range, facet and text search exist; no synonyms, spatial search or relevance feedback.",
    },
    DomainScope {
        domain: "visualization-and-dashboards",
        status: Status::ImplementedNotFullyTested,
        note: "Summary dashboards only; no chart definitions or image export.",
    },
    DomainScope {
        domain: "reporting-and-publishing",
        status: Status::ImplementedNotFullyTested,
        note: "CSV/JSON/GeoJSON exports only; no report templates or PDF.",
    },
    DomainScope {
        domain: "research-notebooks-and-projects",
        status: Status::ImplementedNotFullyTested,
        note: "Project and notebook metadata only; no notebook execution.",
    },
    DomainScope {
        domain: "notifications-and-subscriptions",
        status: Status::ImplementedNotFullyTested,
        note: "Rules, deduplication, outbox and audit exist; email and webhook delivery need an external delivery worker.",
    },
    DomainScope {
        domain: "multilingual-and-localization",
        status: Status::ImplementedNotFullyTested,
        note: "UI labels in English and Hindi only; no localised reports or calendars.",
    },
    DomainScope {
        domain: "privacy-and-data-rights",
        status: Status::ImplementedNotFullyTested,
        note: "Deletion and legal hold exist; no anonymisation, pseudonymisation or export-request workflow.",
    },
    DomainScope {
        domain: "security-operations",
        status: Status::ImplementedNotFullyTested,
        note: "Dependency, licence and secret scans are defined in CI but have not run in this environment.",
    },
    DomainScope {
        domain: "deployment-and-infrastructure",
        status: Status::ImplementedNotFullyTested,
        note: "Container and compose definitions exist; no rollout automation or image signing.",
    },
    DomainScope {
        domain: "licensing-and-intellectual-property",
        status: Status::ImplementedNotFullyTested,
        note: "tools/sbom.mjs produces an SBOM and licence report from lockfiles; dataset and model licence checks are records only.",
    },
    DomainScope {
        domain: "developer-experience-and-sdks",
        status: Status::ImplementedNotFullyTested,
        note: "Rust crates and the Angular API client exist; no published Rust, TypeScript or Python SDK packages.",
    },
    DomainScope {
        domain: "reliability-and-resilience",
        status: Status::ImplementedNotFullyTested,
        note: "Health endpoint and crash-safe journal exist; no separate readiness probe, circuit breakers or failover tests.",
    },
];

/// Future ASPF products: their contract capabilities are implemented and tested in
/// `apsi-contracts`; everything that needs the product itself is blocked.
pub struct ProductContract {
    pub domain: &'static str,
    pub module: &'static str,
    pub product_tests: &'static [&'static str],
}

pub const PRODUCT_CONTRACTS: [ProductContract; 6] = [
    ProductContract { domain: "future-agriql-integration-contract", module: "agriql", product_tests: &["apsi-contracts::contracts::query_contract_rules"] },
    ProductContract {
        domain: "future-kutajkutambi-ai-integration-contract",
        module: "kutajkutambi",
        product_tests: &["apsi-contracts::contracts::inference_contract_rules"],
    },
    ProductContract {
        domain: "future-agricuda-integration-contract",
        module: "agricuda",
        product_tests: &["apsi-contracts::contracts::compute_job_rules", "apsi-contracts::contracts::cpu_fallback_is_the_compute_reference"],
    },
    ProductContract {
        domain: "future-apsnetwork-integration-contract",
        module: "apsnetwork",
        product_tests: &["apsi-contracts::contracts::telemetry_contract_rules"],
    },
    ProductContract {
        domain: "future-nityaga-rtos-integration-contract",
        module: "nityaga",
        product_tests: &["apsi-contracts::contracts::device_command_safety_envelope_rules"],
    },
    ProductContract {
        domain: "hardware-and-fpga-research-interfaces",
        module: "accelerator",
        product_tests: &["apsi-contracts::contracts::accelerator_manifest_rules"],
    },
];

/// Capabilities a contract alone can satisfy.
pub const CONTRACT_CAPABILITIES: [&str; 10] = [
    "schema-contract",
    "metadata-model",
    "versioning",
    "failure-handling",
    "api-endpoint",
    "cli-support",
    "test-coverage",
    "documentation",
    "interoperability",
    "acceptance-evidence",
];

pub const CONTRACT_TESTS: [&str; 6] = [
    "apsi-contracts::contracts::unavailable_adapters_pass_conformance_and_degrade_gracefully",
    "apsi-contracts::contracts::reference_mocks_pass_conformance",
    "apsi-contracts::contracts::broken_mocks_fail_conformance",
    "apsi-contracts::contracts::published_wire_examples_match_contract_types",
    "apsi-contracts::common::tests::version_compatibility_rules",
    "apsi-server::api::api_integrations_report_unavailable_products",
];

pub const CONTRACT_GAP: &str = "Contract only: the product is planned and not yet created; the default adapter reports it unavailable.";

#[derive(Debug, Clone, Serialize)]
pub struct TraceRow {
    pub feature_id: String,
    pub domain: String,
    pub domain_slug: String,
    pub capability: String,
    pub status: Status,
    pub status_label: String,
    pub gap: String,
    pub entity_kinds: Vec<String>,
    pub engine: Vec<String>,
    pub api: Vec<String>,
    pub cli: Vec<String>,
    pub ui: String,
    pub tests: Vec<String>,
    pub docs: String,
}

fn join_gap(a: &str, b: &str) -> String {
    match (a.is_empty(), b.is_empty()) {
        (true, _) => b.to_string(),
        (_, true) => a.to_string(),
        _ => format!("{a} {b}"),
    }
}

fn build(d: &Domain, c: &Capability) -> TraceRow {
    let ev = EVIDENCE.iter().find(|e| e.capability == c.slug).expect("evidence exists for every capability");
    let sub = |s: &str| s.replace("{domain}", &d.slug);
    let mut row = TraceRow {
        feature_id: format!("APSI-F{:04}", (d.index - 1) * catalog().capabilities.len() + c.index),
        domain: d.name.clone(),
        domain_slug: d.slug.clone(),
        capability: c.title.clone(),
        status: ev.status,
        status_label: String::new(),
        gap: ev.gap.to_string(),
        entity_kinds: d.entities.iter().map(|e| e.slug.clone()).collect(),
        engine: ev.engine.iter().map(|s| s.to_string()).collect(),
        api: ev.api.iter().map(|s| sub(s)).collect(),
        cli: ev.cli.iter().map(|s| s.to_string()).collect(),
        ui: ev.ui.into(),
        tests: ev.tests.iter().map(|s| s.to_string()).collect(),
        docs: sub(ev.docs),
    };
    if let Some(pc) = PRODUCT_CONTRACTS.iter().find(|p| p.domain == d.slug) {
        if CONTRACT_CAPABILITIES.contains(&c.slug.as_str()) {
            row.status = Status::ImplementedAndTested;
            row.gap = CONTRACT_GAP.into();
            row.engine = vec![format!("apsi-contracts::{}", pc.module), "apsi-contracts::conformance".into(), "apsi-contracts::common::ContractVersion".into()];
            row.api = vec!["GET /api/v1/integrations".into()];
            row.cli = vec!["apsi integrations".into()];
            row.ui = "-".into();
            row.tests = pc.product_tests.iter().chain(CONTRACT_TESTS.iter()).map(|s| s.to_string()).collect();
            row.docs = format!("70_connectors_external/aspf-contracts/{}.md", pc.module);
        } else {
            row.status = Status::BlockedExternal;
            row.gap = "Needs the product itself, which is planned and not yet created. Records of these entity kinds are still managed generically.".into();
        }
    } else if let Some(scope) = DOMAIN_SCOPES.iter().find(|s| s.domain == d.slug)
        && !UNCAPPED.contains(&c.slug.as_str())
    {
        row.status = row.status.max(scope.status);
        row.gap = join_gap(&row.gap, scope.note);
    }
    row.status_label = row.status.label().into();
    row
}

pub fn rows() -> Vec<TraceRow> {
    let c = catalog();
    c.domains.iter().flat_map(|d| c.capabilities.iter().map(move |cap| build(d, cap))).collect()
}

pub fn row(feature_id: &str) -> Option<TraceRow> {
    let (d, c) = catalog().feature_coordinates(feature_id)?;
    Some(build(d, c))
}

/// Number of features per status, in [`Status::ALL`] order.
pub fn status_counts() -> Vec<(Status, usize)> {
    let rows = rows();
    Status::ALL.iter().map(|s| (*s, rows.iter().filter(|r| r.status == *s).count())).collect()
}

pub fn csv() -> String {
    let cell = |s: &str| if s.contains([',', '"', '\n']) { format!("\"{}\"", s.replace('"', "\"\"")) } else { s.to_string() };
    let mut out = String::from("feature_id,domain,capability,status,gap,entity_kinds,engine,api,cli,ui,tests,docs\r\n");
    for r in rows() {
        let cells = [
            r.feature_id,
            r.domain,
            r.capability,
            r.status_label,
            r.gap,
            r.entity_kinds.join(" "),
            r.engine.join("; "),
            r.api.join("; "),
            r.cli.join("; "),
            r.ui,
            r.tests.join("; "),
            r.docs,
        ];
        out.push_str(&cells.iter().map(|c| cell(c)).collect::<Vec<_>>().join(","));
        out.push_str("\r\n");
    }
    out
}

/// `docs/STATUS.md`: honest completion metrics.
pub fn status_markdown() -> String {
    use std::fmt::Write as _;
    let c = catalog();
    let rows = rows();
    let mut s = String::from("# Feature status\n\n> Generated by `apsi docs generate` from `crates/apsi-core/src/traceability.rs`. Do not edit by hand.\n\n");
    let _ = writeln!(
        s,
        "Every feature in the register has exactly one status. A status reflects code plus automated tests; it is never raised because a directory, interface, mock or document exists. The full matrix, with the gap and evidence for each feature, is [traceability.csv](traceability.csv).\n"
    );
    let _ = writeln!(s, "| Status | Features | Share |\n|---|---|---|");
    for (st, n) in status_counts() {
        let _ = writeln!(s, "| {} | {n} | {:.1}% |", st.label(), n as f64 * 100.0 / rows.len() as f64);
    }
    let _ = writeln!(s, "| **total** | **{}** | 100% |\n", rows.len());
    let _ = writeln!(s, "## By domain\n\n| # | Domain | Tested | Not fully tested | Specified only | Blocked | Scope note |\n|---|---|---|---|---|---|---|");
    for d in &c.domains {
        let mine: Vec<_> = rows.iter().filter(|r| r.domain_slug == d.slug).collect();
        let n = |st| mine.iter().filter(|r| r.status == st).count();
        let note = DOMAIN_SCOPES.iter().find(|x| x.domain == d.slug).map(|x| x.note).unwrap_or_else(|| {
            if PRODUCT_CONTRACTS.iter().any(|p| p.domain == d.slug) {
                "Planned ASPF product: contract implemented and tested; product not yet created."
            } else {
                ""
            }
        });
        let _ = writeln!(
            s,
            "| {} | [{}](domains/{}.md) | {} | {} | {} | {} | {} |",
            d.index,
            d.name,
            d.slug,
            n(Status::ImplementedAndTested),
            n(Status::ImplementedNotFullyTested),
            n(Status::SpecifiedOnly),
            n(Status::BlockedExternal),
            note
        );
    }
    let _ = writeln!(s, "\n## Capability gaps that apply to every domain\n");
    for e in EVIDENCE.iter().filter(|e| !e.gap.is_empty()) {
        let _ = writeln!(s, "- **{}** ({}): {}", e.capability, e.status.label(), e.gap);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_feature_has_evidence() {
        let rows = rows();
        assert_eq!(rows.len(), 3100);
        for (r, f) in rows.iter().zip(crate::catalog::features()) {
            assert_eq!(r.feature_id, f.id);
            assert_eq!(r.domain, f.domain);
            assert_eq!(r.capability, f.title);
            assert!(!r.tests.is_empty() && !r.engine.is_empty(), "{}", r.feature_id);
            assert!(r.api.iter().all(|a| !a.contains("{domain}")));
            assert_eq!(r.status_label, r.status.label());
            // Anything short of fully tested must say what is missing.
            assert!(r.status == Status::ImplementedAndTested || !r.gap.is_empty(), "{} has no gap note", r.feature_id);
        }
        assert_eq!(csv().lines().count(), 3101);
        assert_eq!(EVIDENCE.iter().map(|e| e.capability).collect::<Vec<_>>(), catalog().capabilities.iter().map(|c| c.slug.as_str()).collect::<Vec<_>>());
        assert_eq!(status_counts().iter().map(|(_, n)| n).sum::<usize>(), 3100);
        assert!(EVIDENCE.iter().all(|e| (e.status == Status::ImplementedAndTested) == e.gap.is_empty()));
    }

    #[test]
    fn scopes_reference_real_domains_and_never_raise_status() {
        let slugs: Vec<_> = catalog().domains.iter().map(|d| d.slug.as_str()).collect();
        for s in DOMAIN_SCOPES {
            assert!(slugs.contains(&s.domain), "unknown domain {}", s.domain);
            assert_ne!(s.status, Status::ImplementedAndTested);
            assert!(!s.note.is_empty());
            assert_eq!(DOMAIN_SCOPES.iter().filter(|x| x.domain == s.domain).count(), 1);
        }
        for p in &PRODUCT_CONTRACTS {
            assert!(slugs.contains(&p.domain));
            assert!(DOMAIN_SCOPES.iter().all(|s| s.domain != p.domain));
        }
        for c in CONTRACT_CAPABILITIES.iter().chain(UNCAPPED.iter()) {
            assert!(EVIDENCE.iter().any(|e| e.capability == *c), "{c}");
        }
        let caps = &catalog().capabilities;
        for r in rows() {
            let cap = caps.iter().find(|c| c.title == r.capability).unwrap();
            let base = EVIDENCE.iter().find(|e| e.capability == cap.slug).unwrap().status;
            let is_product = PRODUCT_CONTRACTS.iter().any(|p| p.domain == r.domain_slug);
            if is_product {
                // Planned products are never reported as working products.
                let expected = if CONTRACT_CAPABILITIES.contains(&cap.slug.as_str()) { Status::ImplementedAndTested } else { Status::BlockedExternal };
                assert_eq!(r.status, expected, "{}", r.feature_id);
            } else {
                assert!(r.status >= base, "{}: scope raised a status", r.feature_id);
            }
        }
    }

    /// Every Rust test named anywhere in the matrix must exist in the named crate.
    #[test]
    fn every_referenced_test_exists() {
        let sources: [(&str, &str); 14] = [
            ("apsi-core::engine", include_str!("../tests/engine.rs")),
            ("apsi-core::error", include_str!("error.rs")),
            ("apsi-core::ids", include_str!("ids.rs")),
            ("apsi-core::auth", include_str!("auth.rs")),
            ("apsi-core::schema", include_str!("schema.rs")),
            ("apsi-core::validation", include_str!("validation.rs")),
            ("apsi-core::record", include_str!("record.rs")),
            ("apsi-core::search", include_str!("search.rs")),
            ("apsi-core::exchange", include_str!("exchange.rs")),
            ("apsi-core::metrics", include_str!("metrics.rs")),
            ("apsi-core::audit", include_str!("audit.rs")),
            ("apsi-core::migration", include_str!("migration.rs")),
            ("apsi-core::traceability", include_str!("traceability.rs")),
            (
                "apsi-science::*",
                concat!(
                    include_str!("../../apsi-science/src/stats.rs"),
                    include_str!("../../apsi-science/src/geo.rs"),
                    include_str!("../../apsi-science/src/agronomy.rs"),
                    include_str!("../../apsi-science/src/registry.rs"),
                    include_str!("../../apsi-science/src/units.rs"),
                ),
            ),
        ];
        let server = include_str!("../../apsi-server/tests/api.rs").to_string() + include_str!("../../apsi-server/src/auth.rs");
        let cli = include_str!("../../apsi-cli/tests/cli.rs");
        let contracts = include_str!("../../apsi-contracts/tests/contracts.rs").to_string() + include_str!("../../apsi-contracts/src/common.rs");
        let mut all: Vec<String> = rows().into_iter().flat_map(|r| r.tests).collect();
        all.sort();
        all.dedup();
        for t in &all {
            if t.starts_with("web::") {
                continue;
            }
            let name = t.rsplit("::").next().unwrap();
            let needle = format!("fn {name}(");
            let found = if t.starts_with("apsi-server") {
                server.contains(&needle)
            } else if t.starts_with("apsi-cli") {
                cli.contains(&needle)
            } else if t.starts_with("apsi-contracts") {
                contracts.contains(&needle)
            } else if t.starts_with("apsi-science") {
                sources[13].1.contains(&needle)
            } else {
                sources.iter().any(|(prefix, src)| t.starts_with(prefix) && src.contains(&needle))
            };
            assert!(found, "matrix references missing test {t}");
        }
    }
}
