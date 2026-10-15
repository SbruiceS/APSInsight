//! End-to-end tests of the `apsi` binary: structured output, exit codes, files.

use serde_json::Value;
use std::path::Path;
use std::process::Command;

const D: &str = "soil-chemistry-and-fertility";
const E: &str = "macro-nutrients";

fn apsi(dir: &Path, args: &[&str]) -> (i32, Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_apsi")).arg("--data-dir").arg(dir).args(["--tenant", "lab"]).args(args).output().expect("binary runs");
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let v = serde_json::from_str(&text).unwrap_or(Value::String(text));
    (out.status.code().unwrap_or(-1), v)
}

#[test]
fn cli_json_output_and_exit_codes() {
    let dir = tempfile::tempdir().unwrap();
    let (code, v) = apsi(dir.path(), &["records", "create", D, E, "--json", r#"{"name":"N-P-K plot 3","attributes":{"value":42.5,"unit":"kg/ha"}}"#]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["ok"], true);
    let id = v["data"]["id"].as_str().unwrap().to_string();
    // stable exit codes per error family
    let (code, v) = apsi(dir.path(), &["records", "create", D, E, "--json", r#"{"name":""}"#]);
    assert_eq!((code, v["error"]["code"].as_str()), (3, Some("validation_failed")));
    let (code, _) = apsi(dir.path(), &["records", "get", "rec_00000000000000000000000000000000"]);
    assert_eq!(code, 4);
    let (code, v) = apsi(dir.path(), &["records", "update", &id, "--expected-version", "7", "--json", r#"{"name":"x"}"#]);
    assert_eq!((code, v["error"]["retryable"].as_bool()), (5, Some(true)));
    let (code, _) = apsi(dir.path(), &["--role", "viewer", "records", "create", D, E, "--json", r#"{"name":"x"}"#]);
    assert_eq!(code, 7);
    let (code, _) = apsi(dir.path(), &["records", "create", D, E, "--json", "{oops"]);
    assert_eq!(code, 2);
    let (code, _) = apsi(dir.path(), &["no-such-command"]);
    assert_eq!(code, 2, "clap usage errors exit with 2");
    // state persisted across invocations
    let (code, v) = apsi(dir.path(), &["records", "search", D, "--entity", E, "--filter", "unit:eq:kg/ha"]);
    assert_eq!((code, v["data"]["total"].as_u64()), (0, Some(1)));
    let (_, v) = apsi(dir.path(), &["audit", "verify"]);
    assert_eq!(v["data"]["valid"], true);
}

#[test]
fn cli_import_export_and_features() {
    let dir = tempfile::tempdir().unwrap();
    let csv = dir.path().join("in.csv");
    std::fs::write(&csv, "name,value,unit,tags\nA,10,kg/ha,x|y\nB,7.5,kg/ha,\n,5,kg/ha,\n").unwrap();
    let (code, v) = apsi(dir.path(), &["records", "import", D, E, "--file", csv.to_str().unwrap(), "--dry-run"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!((v["data"]["created"].as_u64(), v["data"]["failed"].as_u64()), (Some(2), Some(1)));
    apsi(dir.path(), &["records", "import", D, E, "--file", csv.to_str().unwrap()]);
    let out = dir.path().join("out.json");
    let (code, v) = apsi(dir.path(), &["records", "export", D, E, "--format", "json", "--out", out.to_str().unwrap()]);
    assert_eq!(code, 0, "{v}");
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
    assert_eq!(doc["header"]["count"], 2);
    assert_eq!(doc["header"]["encoding"], "utf-8");
    let (_, v) = apsi(dir.path(), &["features", "list", "--domain", D]);
    assert_eq!(v["data"]["count"], 31);
    let (_, v) = apsi(dir.path(), &["features", "show", "APSI-F0001"]);
    assert_eq!(v["data"]["evidence"]["capability"], "Registration and identity");
    let (_, v) = apsi(dir.path(), &["compute", "agro.usda_texture", "--json", r#"{"sand":40,"silt":40,"clay":20}"#]);
    assert_eq!(v["data"], "loam");
    let (code, _) = apsi(dir.path(), &["compute", "geo.area", "--json", r#"{"ring":[]}"#]);
    assert_eq!(code, 2);
}

#[test]
fn cli_bench_reports_measured_numbers() {
    let dir = tempfile::tempdir().unwrap();
    let (code, v) = apsi(dir.path(), &["bench", "--records", "200"]);
    assert_eq!(code, 0, "{v}");
    let results = v["data"]["results"].as_array().unwrap();
    assert_eq!(results.len(), 4);
    assert!(results.iter().all(|r| r["seconds"].as_f64().unwrap() >= 0.0 && r["ops"].as_u64().unwrap() > 0));
    assert_eq!(v["data"]["workload"]["records"], 200);
    let (code, _) = apsi(dir.path(), &["bench", "--records", "0"]);
    assert_eq!(code, 2);
}

/// `docs/` is generated; this fails when someone changes the code but forgets to
/// run `apsi docs generate`.
#[test]
fn generated_docs_are_up_to_date() {
    let docs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs");
    let read = |p: &Path| {
        std::fs::read_to_string(p).unwrap_or_else(|_| panic!("missing {} — run `cargo run -p apsi-cli -- docs generate`", p.display())).replace("\r\n", "\n")
    };
    for d in &apsi_core::catalog::catalog().domains {
        let path = docs.join("domains").join(format!("{}.md", d.slug));
        assert_eq!(read(&path), apsi_core::docs::domain_markdown(d).replace("\r\n", "\n"), "{} is stale", path.display());
    }
    assert_eq!(read(&docs.join("domains/README.md")), apsi_core::docs::index_markdown());
    assert_eq!(read(&docs.join("traceability.csv")), apsi_core::traceability::csv().replace("\r\n", "\n"));
    assert_eq!(read(&docs.join("STATUS.md")), apsi_core::traceability::status_markdown());
    assert_eq!(read(&docs.join("UNITS.md")), apsi_science::units::catalogue_markdown());
}
