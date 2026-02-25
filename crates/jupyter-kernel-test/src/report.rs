use crate::types::{ConformanceMatrix, KernelReport, TestOutcome};

/// Render a single kernel report as a Markdown summary.
pub fn render_kernel_markdown(report: &KernelReport) -> String {
    let mut out = String::new();

    out.push_str(&format!("## {}\n\n", report.display_name));
    out.push_str(&format!("- **Kernel**: `{}`\n", report.kernel_name));
    out.push_str(&format!("- **Language**: {}\n", report.language));
    if let Some(v) = &report.protocol_version {
        out.push_str(&format!("- **Protocol version**: {v}\n"));
    }
    if let Some(i) = &report.implementation {
        out.push_str(&format!("- **Implementation**: {i}\n"));
    }
    out.push_str(&format!(
        "- **Score**: {}/{}\n",
        report.pass_count(),
        report.total_count()
    ));
    out.push_str(&format!(
        "- **Duration**: {:.1}s\n\n",
        report.total_duration.as_secs_f64()
    ));

    // Score by tier
    out.push_str("### Score by tier\n\n");
    out.push_str("| Tier | Passed | Total | % |\n");
    out.push_str("|------|--------|-------|---|\n");
    for (tier, (pass, total)) in report.score_by_tier() {
        let pct = if total > 0 {
            (pass as f64 / total as f64 * 100.0) as u32
        } else {
            0
        };
        out.push_str(&format!("| {tier} | {pass} | {total} | {pct}% |\n"));
    }
    out.push('\n');

    // Detailed results
    out.push_str("### Detailed results\n\n");
    out.push_str("| Test | Category | Result | Duration |\n");
    out.push_str("|------|----------|--------|----------|\n");
    for r in &report.results {
        let result_str = match &r.outcome {
            TestOutcome::Pass => format!("{} Pass", r.outcome.emoji()),
            TestOutcome::Fail { reason } => {
                format!("{} Fail: {}", r.outcome.emoji(), truncate(reason, 60))
            }
            TestOutcome::Timeout => format!("{} Timeout", r.outcome.emoji()),
            TestOutcome::Skipped { reason } => {
                format!("{} Skip: {}", r.outcome.emoji(), truncate(reason, 40))
            }
        };
        out.push_str(&format!(
            "| {} | {} | {} | {:.0?} |\n",
            r.test_name,
            r.category.description(),
            result_str,
            r.duration
        ));
    }
    out.push('\n');

    out
}

/// Render the full conformance matrix comparing multiple kernels.
pub fn render_matrix_markdown(matrix: &ConformanceMatrix) -> String {
    if matrix.reports.is_empty() {
        return "No kernel reports to display.\n".to_string();
    }

    let mut out = String::new();

    out.push_str("# Jupyter Kernel Protocol Conformance Matrix\n\n");
    out.push_str(&format!("Generated: {}\n\n", matrix.timestamp));

    // Collect all test names (use first report as the reference)
    let all_test_names: Vec<&str> = matrix.reports[0]
        .results
        .iter()
        .map(|r| r.test_name.as_str())
        .collect();

    // Header row
    out.push_str("| Test |");
    for report in &matrix.reports {
        out.push_str(&format!(" {} |", report.display_name));
    }
    out.push('\n');

    // Separator
    out.push_str("|------|");
    for _ in &matrix.reports {
        out.push_str("------|");
    }
    out.push('\n');

    // Data rows
    for test_name in &all_test_names {
        out.push_str(&format!("| `{test_name}` |"));
        for report in &matrix.reports {
            let result = report
                .results
                .iter()
                .find(|r| r.test_name == *test_name);
            let symbol = match result {
                Some(r) => r.outcome.emoji(),
                None => "—",
            };
            out.push_str(&format!(" {symbol} |"));
        }
        out.push('\n');
    }

    // Summary row
    out.push_str("| **Score** |");
    for report in &matrix.reports {
        out.push_str(&format!(
            " **{}/{}** |",
            report.pass_count(),
            report.total_count()
        ));
    }
    out.push('\n');
    out.push('\n');

    // Per-kernel details
    for report in &matrix.reports {
        out.push_str(&render_kernel_markdown(report));
    }

    out
}

/// Render a concise terminal-friendly summary.
pub fn render_terminal_summary(report: &KernelReport) -> String {
    let mut out = String::new();

    out.push_str(&format!(
        "\n{} ({}) — {}/{} passed\n",
        report.display_name,
        report.kernel_name,
        report.pass_count(),
        report.total_count()
    ));

    for (tier, (pass, total)) in report.score_by_tier() {
        let bar_len = 20;
        let filled = if total > 0 {
            (pass as f64 / total as f64 * bar_len as f64) as usize
        } else {
            0
        };
        let bar: String = "█".repeat(filled) + &"░".repeat(bar_len - filled);
        out.push_str(&format!(
            "  Tier {tier}: [{bar}] {pass}/{total}\n"
        ));
    }

    // Show failures
    let failures: Vec<_> = report
        .results
        .iter()
        .filter(|r| matches!(&r.outcome, TestOutcome::Fail { .. }))
        .collect();

    if !failures.is_empty() {
        out.push_str("\n  Failures:\n");
        for f in &failures {
            if let TestOutcome::Fail { reason } = &f.outcome {
                out.push_str(&format!("    {} — {}\n", f.test_name, reason));
            }
        }
    }

    let timeouts: Vec<_> = report
        .results
        .iter()
        .filter(|r| matches!(&r.outcome, TestOutcome::Timeout))
        .collect();

    if !timeouts.is_empty() {
        out.push_str("\n  Timeouts:\n");
        for t in &timeouts {
            out.push_str(&format!("    {}\n", t.test_name));
        }
    }

    out.push_str(&format!(
        "\n  Total time: {:.1}s\n",
        report.total_duration.as_secs_f64()
    ));

    out
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}
