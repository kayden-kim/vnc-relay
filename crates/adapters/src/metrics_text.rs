//! Prometheus text 0.0.4 encoding of a shared metrics snapshot.
//!
//! See [metrics-v1 §1.6](../../../docs/contracts/metrics-v1.md#16-prometheus-텍스트-표현)
//! and the [metrics design](../../../docs/architectures/metrics.md).

use std::fmt::Write as _;

use vnc_relay_application::metrics::snapshot::{
    MetricKind, MetricPoint, MetricsSnapshot, Number, PointValue,
};

/// Encodes families and points in snapshot order, without timestamps or I/O.
/// See metrics-v1 §1.6.
pub fn encode(snapshot: &MetricsSnapshot) -> String {
    let mut output = String::with_capacity(estimated_capacity(snapshot));
    output.push_str("# HELP target_info Target metadata\n# TYPE target_info gauge\n");
    output.push_str("target_info{service_name=\"vnc-relay\",service_version=\"");
    push_escaped(&mut output, snapshot.service_version);
    output.push_str("\"} 1\n");

    let longest_name = snapshot
        .families
        .iter()
        .map(|family| family.name.len())
        .max()
        .unwrap_or(0);
    let mut name = String::with_capacity(longest_name + "_seconds_total".len());
    for family in &snapshot.families {
        name.clear();
        push_name(&mut name, family.name);
        name.push_str(unit_suffix(family.unit));
        if family.kind == MetricKind::Counter {
            name.push_str("_total");
        }
        let _ = writeln!(output, "# HELP {name} {}", family.help);
        let _ = writeln!(output, "# TYPE {name} {}", prometheus_kind(family.kind));
        if !family.available {
            continue;
        }
        for point in &family.points {
            match &point.value {
                PointValue::Number(value) => {
                    output.push_str(&name);
                    push_labels(&mut output, point, None);
                    output.push(' ');
                    push_number(&mut output, *value);
                    output.push('\n');
                }
                PointValue::Histogram {
                    count,
                    sum,
                    bucket_counts,
                } => {
                    debug_assert_eq!(bucket_counts.len(), family.bounds.len() + 1);
                    let mut cumulative = 0;
                    for (bound, bucket_count) in family.bounds.iter().zip(bucket_counts) {
                        cumulative += bucket_count;
                        output.push_str(&name);
                        output.push_str("_bucket");
                        push_labels(&mut output, point, Some(*bound));
                        let _ = writeln!(output, " {cumulative}");
                    }
                    output.push_str(&name);
                    output.push_str("_bucket");
                    push_labels(&mut output, point, Some(f64::INFINITY));
                    let _ = writeln!(output, " {count}");
                    output.push_str(&name);
                    output.push_str("_sum");
                    push_labels(&mut output, point, None);
                    output.push(' ');
                    push_number(&mut output, Number::Float(*sum));
                    output.push('\n');
                    output.push_str(&name);
                    output.push_str("_count");
                    push_labels(&mut output, point, None);
                    let _ = writeln!(output, " {count}");
                }
            }
        }
    }
    output
}

fn prometheus_kind(kind: MetricKind) -> &'static str {
    match kind {
        MetricKind::Counter => "counter",
        MetricKind::UpDownCounter | MetricKind::Gauge => "gauge",
        MetricKind::Histogram => "histogram",
    }
}

fn unit_suffix(unit: &str) -> &'static str {
    match unit {
        "s" => "_seconds",
        "By" => "_bytes",
        _ => "",
    }
}

fn push_name(output: &mut String, name: &str) {
    for (index, part) in name.split('.').enumerate() {
        if index != 0 {
            output.push('_');
        }
        output.push_str(part);
    }
}

fn push_escaped(output: &mut String, value: &str) {
    let mut start = 0;
    for (index, byte) in value.bytes().enumerate() {
        let escaped = match byte {
            b'\\' => "\\\\",
            b'\"' => "\\\"",
            b'\n' => "\\n",
            _ => continue,
        };
        output.push_str(&value[start..index]);
        output.push_str(escaped);
        start = index + 1;
    }
    output.push_str(&value[start..]);
}

fn push_labels(output: &mut String, point: &MetricPoint, bound: Option<f64>) {
    if point.attributes.is_empty() && bound.is_none() {
        return;
    }
    output.push('{');
    for (index, (name, value)) in point.attributes.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        push_name(output, name);
        output.push_str("=\"");
        push_escaped(output, value);
        output.push('"');
    }
    if let Some(bound) = bound {
        if !point.attributes.is_empty() {
            output.push(',');
        }
        output.push_str("le=\"");
        if bound == f64::INFINITY {
            output.push_str("+Inf");
        } else {
            push_number(output, Number::Float(bound));
        }
        output.push('"');
    }
    output.push('}');
}

fn push_number(output: &mut String, value: Number) {
    match value {
        Number::Int(value) => {
            let _ = write!(output, "{value}");
        }
        Number::Float(value) => {
            debug_assert!(value.is_finite());
            let _ = write!(output, "{value}");
        }
    }
}

fn estimated_capacity(snapshot: &MetricsSnapshot) -> usize {
    let mut capacity = 160 + snapshot.service_version.len() * 2;
    for family in &snapshot.families {
        let name_len = family.name.len() + "_seconds_total".len();
        capacity += name_len * 2 + family.help.len() + 32;
        if !family.available {
            continue;
        }
        for point in &family.points {
            let labels_len: usize = point
                .attributes
                .iter()
                .map(|(name, value)| name.len() + value.len() * 2 + 4)
                .sum();
            let lines = match &point.value {
                PointValue::Number(_) => 1,
                PointValue::Histogram { .. } => family.bounds.len() + 3,
            };
            // Leave room for numeric values, histogram suffixes and `le` labels.
            capacity += lines * (name_len + labels_len + 64);
        }
    }
    capacity
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::SystemTime;

    use super::*;
    use vnc_relay_application::metrics::snapshot::MetricFamily;

    const TARGET: &str = concat!(
        "# HELP target_info Target metadata\n",
        "# TYPE target_info gauge\n",
        "target_info{service_name=\"vnc-relay\",service_version=\"2026.11.6\"} 1\n",
    );

    fn snapshot(families: Vec<MetricFamily>) -> MetricsSnapshot {
        MetricsSnapshot {
            as_of: SystemTime::UNIX_EPOCH,
            service_version: "2026.11.6",
            families,
        }
    }

    fn family(name: &'static str, kind: MetricKind, unit: &'static str) -> MetricFamily {
        MetricFamily {
            name,
            kind,
            unit,
            help: "Sample metric",
            available: true,
            bounds: &[],
            points: Vec::new(),
        }
    }

    fn point(attributes: &[(&'static str, &str)], value: PointValue) -> MetricPoint {
        MetricPoint {
            attributes: attributes
                .iter()
                .map(|(name, value)| (*name, Arc::from(*value)))
                .collect(),
            value,
        }
    }

    /// spec: operations.md:OPS-083 partial
    #[test]
    fn converts_every_contract_family_name_and_kind_in_given_order() {
        use MetricKind::{Counter, Gauge, Histogram, UpDownCounter};

        let cases = [
            (
                "vnc_relay.connection.accepted",
                Counter,
                "{connection}",
                "vnc_relay_connection_accepted_total",
                "counter",
            ),
            (
                "vnc_relay.connection.rejected",
                Counter,
                "{connection}",
                "vnc_relay_connection_rejected_total",
                "counter",
            ),
            (
                "vnc_relay.connection.aborted",
                Counter,
                "{connection}",
                "vnc_relay_connection_aborted_total",
                "counter",
            ),
            (
                "vnc_relay.connection.in_progress",
                UpDownCounter,
                "{connection}",
                "vnc_relay_connection_in_progress",
                "gauge",
            ),
            (
                "vnc_relay.relay_connection.usage",
                UpDownCounter,
                "{connection}",
                "vnc_relay_relay_connection_usage",
                "gauge",
            ),
            (
                "vnc_relay.relay_connection.limit",
                UpDownCounter,
                "{connection}",
                "vnc_relay_relay_connection_limit",
                "gauge",
            ),
            (
                "vnc_relay.admin_connection.usage",
                UpDownCounter,
                "{connection}",
                "vnc_relay_admin_connection_usage",
                "gauge",
            ),
            (
                "vnc_relay.admin_connection.limit",
                UpDownCounter,
                "{connection}",
                "vnc_relay_admin_connection_limit",
                "gauge",
            ),
            (
                "vnc_relay.handshake.duration",
                Histogram,
                "s",
                "vnc_relay_handshake_duration_seconds",
                "histogram",
            ),
            (
                "vnc_relay.waiting.count",
                UpDownCounter,
                "{connection}",
                "vnc_relay_waiting_count",
                "gauge",
            ),
            (
                "vnc_relay.waiting.duration",
                Histogram,
                "s",
                "vnc_relay_waiting_duration_seconds",
                "histogram",
            ),
            (
                "vnc_relay.session.count",
                UpDownCounter,
                "{session}",
                "vnc_relay_session_count",
                "gauge",
            ),
            (
                "vnc_relay.session.started",
                Counter,
                "{session}",
                "vnc_relay_session_started_total",
                "counter",
            ),
            (
                "vnc_relay.session.closed",
                Counter,
                "{session}",
                "vnc_relay_session_closed_total",
                "counter",
            ),
            (
                "vnc_relay.session.duration",
                Histogram,
                "s",
                "vnc_relay_session_duration_seconds",
                "histogram",
            ),
            (
                "vnc_relay.session.io",
                Counter,
                "By",
                "vnc_relay_session_io_bytes_total",
                "counter",
            ),
            (
                "vnc_relay.target_connect.duration",
                Histogram,
                "s",
                "vnc_relay_target_connect_duration_seconds",
                "histogram",
            ),
            (
                "vnc_relay.ticket.accepted",
                Counter,
                "{ticket}",
                "vnc_relay_ticket_accepted_total",
                "counter",
            ),
            (
                "vnc_relay.websocket.ping_timeouts",
                Counter,
                "{connection}",
                "vnc_relay_websocket_ping_timeouts_total",
                "counter",
            ),
            (
                "vnc_relay.admin.logins",
                Counter,
                "{attempt}",
                "vnc_relay_admin_logins_total",
                "counter",
            ),
            (
                "vnc_relay.admin.lockouts",
                Counter,
                "{lockout}",
                "vnc_relay_admin_lockouts_total",
                "counter",
            ),
            (
                "vnc_relay.admin.terminations",
                Counter,
                "{request}",
                "vnc_relay_admin_terminations_total",
                "counter",
            ),
            (
                "vnc_relay.admin_session.count",
                UpDownCounter,
                "{session}",
                "vnc_relay_admin_session_count",
                "gauge",
            ),
            (
                "vnc_relay.config.reloads",
                Counter,
                "{reload}",
                "vnc_relay_config_reloads_total",
                "counter",
            ),
            (
                "vnc_relay.config.edits",
                Counter,
                "{request}",
                "vnc_relay_config_edits_total",
                "counter",
            ),
            (
                "process.uptime",
                Gauge,
                "s",
                "process_uptime_seconds",
                "gauge",
            ),
            (
                "process.cpu.time",
                Counter,
                "s",
                "process_cpu_time_seconds_total",
                "counter",
            ),
            (
                "process.memory.usage",
                UpDownCounter,
                "By",
                "process_memory_usage_bytes",
                "gauge",
            ),
            (
                "process.memory.virtual",
                UpDownCounter,
                "By",
                "process_memory_virtual_bytes",
                "gauge",
            ),
            (
                "process.thread.count",
                UpDownCounter,
                "{thread}",
                "process_thread_count",
                "gauge",
            ),
            (
                "process.unix.file_descriptor.count",
                UpDownCounter,
                "{file_descriptor}",
                "process_unix_file_descriptor_count",
                "gauge",
            ),
            (
                "process.windows.handle.count",
                UpDownCounter,
                "{handle}",
                "process_windows_handle_count",
                "gauge",
            ),
        ];
        let mut families = Vec::new();
        let mut expected = TARGET.to_owned();
        for (otel, kind, unit, prometheus, prometheus_kind) in cases {
            families.push(family(otel, kind, unit));
            let _ = writeln!(expected, "# HELP {prometheus} Sample metric");
            let _ = writeln!(expected, "# TYPE {prometheus} {prometheus_kind}");
        }
        assert_eq!(encode(&snapshot(families)), expected);
    }

    /// spec: operations.md:OPS-083 partial
    #[test]
    fn writes_target_metadata_first_and_ends_with_lf() {
        let text = encode(&snapshot(Vec::new()));
        assert_eq!(text, TARGET);
        assert!(text.ends_with('\n'));
        assert!(!text.contains('\r'));
    }

    #[test]
    fn escapes_label_values_and_preserves_attribute_order() {
        let mut metric = family("process.cpu.time", MetricKind::Counter, "s");
        metric.points.push(point(
            &[("cpu.mode", "ü\\\"\nuser"), ("listener", "wss")],
            PointValue::Number(Number::Float(1.0)),
        ));
        let text = encode(&snapshot(vec![metric]));
        assert_eq!(
            text.strip_prefix(TARGET),
            Some(concat!(
                "# HELP process_cpu_time_seconds_total Sample metric\n",
                "# TYPE process_cpu_time_seconds_total counter\n",
                "process_cpu_time_seconds_total{cpu_mode=\"ü\\\\\\\"\\nuser\",listener=\"wss\"} 1\n",
            )),
        );
    }

    #[test]
    fn escapes_service_version_as_a_label_value() {
        let mut input = snapshot(Vec::new());
        input.service_version = "version\\\"\n";
        assert_eq!(
            encode(&input),
            concat!(
                "# HELP target_info Target metadata\n",
                "# TYPE target_info gauge\n",
                "target_info{service_name=\"vnc-relay\",service_version=\"version\\\\\\\"\\n\"} 1\n",
            ),
        );
    }

    #[test]
    fn omits_empty_label_braces_and_preserves_point_order() {
        let mut metric = family(
            "process.thread.count",
            MetricKind::UpDownCounter,
            "{thread}",
        );
        metric.points = vec![
            point(&[], PointValue::Number(Number::Int(9))),
            point(&[], PointValue::Number(Number::Int(2))),
            point(&[], PointValue::Number(Number::Int(7))),
        ];
        assert_eq!(
            encode(&snapshot(vec![metric])).strip_prefix(TARGET),
            Some(concat!(
                "# HELP process_thread_count Sample metric\n",
                "# TYPE process_thread_count gauge\n",
                "process_thread_count 9\n",
                "process_thread_count 2\n",
                "process_thread_count 7\n",
            )),
        );
    }

    /// spec: operations.md:OPS-093 partial
    #[test]
    fn accumulates_histogram_buckets_and_writes_sum_and_count_per_point() {
        let mut metric = family("vnc_relay.handshake.duration", MetricKind::Histogram, "s");
        metric.bounds = &[0.005, 0.01, 1.0, 86400.0];
        metric.points = vec![
            point(
                &[("listener", "wss"), ("mode", "mode2")],
                PointValue::Histogram {
                    count: 8,
                    sum: 90001.0121,
                    bucket_counts: vec![2, 1, 0, 3, 2],
                },
            ),
            point(
                &[],
                PointValue::Histogram {
                    count: 1,
                    sum: 0.005,
                    bucket_counts: vec![1, 0, 0, 0, 0],
                },
            ),
        ];
        assert_eq!(
            encode(&snapshot(vec![metric])).strip_prefix(TARGET),
            Some(concat!(
                "# HELP vnc_relay_handshake_duration_seconds Sample metric\n",
                "# TYPE vnc_relay_handshake_duration_seconds histogram\n",
                "vnc_relay_handshake_duration_seconds_bucket{listener=\"wss\",mode=\"mode2\",le=\"0.005\"} 2\n",
                "vnc_relay_handshake_duration_seconds_bucket{listener=\"wss\",mode=\"mode2\",le=\"0.01\"} 3\n",
                "vnc_relay_handshake_duration_seconds_bucket{listener=\"wss\",mode=\"mode2\",le=\"1\"} 3\n",
                "vnc_relay_handshake_duration_seconds_bucket{listener=\"wss\",mode=\"mode2\",le=\"86400\"} 6\n",
                "vnc_relay_handshake_duration_seconds_bucket{listener=\"wss\",mode=\"mode2\",le=\"+Inf\"} 8\n",
                "vnc_relay_handshake_duration_seconds_sum{listener=\"wss\",mode=\"mode2\"} 90001.0121\n",
                "vnc_relay_handshake_duration_seconds_count{listener=\"wss\",mode=\"mode2\"} 8\n",
                "vnc_relay_handshake_duration_seconds_bucket{le=\"0.005\"} 1\n",
                "vnc_relay_handshake_duration_seconds_bucket{le=\"0.01\"} 1\n",
                "vnc_relay_handshake_duration_seconds_bucket{le=\"1\"} 1\n",
                "vnc_relay_handshake_duration_seconds_bucket{le=\"86400\"} 1\n",
                "vnc_relay_handshake_duration_seconds_bucket{le=\"+Inf\"} 1\n",
                "vnc_relay_handshake_duration_seconds_sum 0.005\n",
                "vnc_relay_handshake_duration_seconds_count 1\n",
            )),
        );
    }

    /// spec: operations.md:OPS-093 partial
    #[test]
    fn formats_numbers_as_round_trip_decimals_without_exponents() {
        let cases = [
            (Number::Float(0.005), "0.005"),
            (Number::Float(0.0121), "0.0121"),
            (Number::Float(1.0), "1"),
            (Number::Float(86400.0), "86400"),
            (
                Number::Float(0.00000000000000000001),
                "0.00000000000000000001",
            ),
            (
                Number::Float(100000000000000000000.0),
                "100000000000000000000",
            ),
            (Number::Int(0), "0"),
            (Number::Int(u64::MAX), "18446744073709551615"),
        ];
        for (number, expected) in cases {
            let mut text = String::new();
            push_number(&mut text, number);
            assert_eq!(text, expected);
            match number {
                Number::Int(value) => assert_eq!(text.parse::<u64>(), Ok(value)),
                Number::Float(value) => assert_eq!(text.parse::<f64>(), Ok(value)),
            }
        }
    }

    /// spec: operations.md:OPS-094 partial
    #[test]
    fn omits_unavailable_samples_without_affecting_other_families() {
        let mut missing = family("process.memory.usage", MetricKind::UpDownCounter, "By");
        missing.available = false;
        missing.points = vec![point(&[], PointValue::Number(Number::Int(99)))];
        let mut present = family(
            "process.thread.count",
            MetricKind::UpDownCounter,
            "{thread}",
        );
        present.points = vec![point(&[], PointValue::Number(Number::Int(3)))];
        assert_eq!(
            encode(&snapshot(vec![missing, present])).strip_prefix(TARGET),
            Some(concat!(
                "# HELP process_memory_usage_bytes Sample metric\n",
                "# TYPE process_memory_usage_bytes gauge\n",
                "# HELP process_thread_count Sample metric\n",
                "# TYPE process_thread_count gauge\n",
                "process_thread_count 3\n",
            )),
        );
    }
}
