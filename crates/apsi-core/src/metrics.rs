//! Operation metrics (Prometheus text exposition) and log redaction ("Observability").
//!
//! Metric labels only ever contain catalogue slugs, operation names and outcome
//! codes — never record contents, tenants' attribute values or credentials.

use parking_lot::Mutex;
use std::collections::BTreeMap;
use std::fmt::Write as _;

#[derive(Debug, Default)]
pub struct Metrics {
    counters: Mutex<BTreeMap<(String, String, String), u64>>,
    durations: Mutex<BTreeMap<String, (u64, f64)>>,
}

impl Metrics {
    pub fn count(&self, operation: &str, domain: &str, outcome: &str) {
        *self.counters.lock().entry((operation.into(), domain.into(), outcome.into())).or_default() += 1;
    }

    pub fn observe_seconds(&self, operation: &str, seconds: f64) {
        let mut d = self.durations.lock();
        let e = d.entry(operation.into()).or_default();
        e.0 += 1;
        e.1 += seconds;
    }

    pub fn counter(&self, operation: &str, domain: &str, outcome: &str) -> u64 {
        self.counters.lock().get(&(operation.into(), domain.into(), outcome.into())).copied().unwrap_or(0)
    }

    pub fn render(&self) -> String {
        let mut out = String::from("# HELP apsi_operations_total Engine operations by outcome.\n# TYPE apsi_operations_total counter\n");
        for ((op, domain, outcome), v) in self.counters.lock().iter() {
            let _ = writeln!(out, "apsi_operations_total{{operation=\"{op}\",domain=\"{domain}\",outcome=\"{outcome}\"}} {v}");
        }
        out.push_str("# HELP apsi_operation_seconds Time spent per operation.\n# TYPE apsi_operation_seconds summary\n");
        for (op, (n, s)) in self.durations.lock().iter() {
            let _ = writeln!(out, "apsi_operation_seconds_count{{operation=\"{op}\"}} {n}");
            let _ = writeln!(out, "apsi_operation_seconds_sum{{operation=\"{op}\"}} {s:.6}");
        }
        out
    }
}

const SENSITIVE: [&str; 6] = ["token", "password", "secret", "authorization", "api_key", "apikey"];

/// Masks values following sensitive keys (`key=value`, `key: value`, `"key":"value"`)
/// so they never reach logs or the audit log.
pub fn redact(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let lower = text.to_ascii_lowercase();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let hit = SENSITIVE.iter().find(|k| lower[i..].starts_with(*k));
        if let Some(k) = hit {
            let mut j = i + k.len();
            out.push_str(&text[i..j]);
            while j < bytes.len() && matches!(bytes[j], b'"' | b'\'' | b' ' | b':' | b'=') {
                out.push(bytes[j] as char);
                j += 1;
            }
            let start = j;
            let stop = |b: u8| matches!(b, b'"' | b'\'' | b' ' | b',' | b'&' | b'}' | b';' | b'\n');
            while j < bytes.len() && !stop(bytes[j]) {
                j += 1;
            }
            // `Authorization: Bearer <credential>` — the scheme word is followed by the secret.
            let word = lower[start..j].to_string();
            if (word == "bearer" || word == "basic") && j < bytes.len() && bytes[j] == b' ' {
                j += 1;
                while j < bytes.len() && !stop(bytes[j]) {
                    j += 1;
                }
            }
            if j > start {
                out.push_str("[REDACTED]");
            }
            i = j;
        } else {
            let ch = text[i..].chars().next().expect("in bounds");
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redaction() {
        assert_eq!(redact("Authorization: Bearer"), "Authorization: [REDACTED]");
        assert_eq!(redact("Authorization: Bearer s3cr3t-value done"), "Authorization: [REDACTED] done");
        assert_eq!(redact("token=abc&x=1"), "token=[REDACTED]&x=1");
        assert_eq!(redact(r#"{"password":"hunter2","n":1}"#), r#"{"password":"[REDACTED]","n":1}"#);
        assert_eq!(redact("nothing here ü"), "nothing here ü");
    }

    #[test]
    fn counters_render() {
        let m = Metrics::default();
        m.count("create", "soil", "ok");
        m.count("create", "soil", "ok");
        m.observe_seconds("create", 0.5);
        assert_eq!(m.counter("create", "soil", "ok"), 2);
        let text = m.render();
        assert!(text.contains("apsi_operations_total{operation=\"create\",domain=\"soil\",outcome=\"ok\"} 2"));
        assert!(text.contains("apsi_operation_seconds_count{operation=\"create\"} 1"));
    }
}
