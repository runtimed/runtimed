use clap::Parser;
use jupyter_kernel_test::harness::run_conformance_suite;
use jupyter_kernel_test::report;
use jupyter_kernel_test::tests;
use jupyter_kernel_test::types::ConformanceMatrix;

/// Jupyter Kernel Protocol Conformance Test Suite
///
/// Tests installed Jupyter kernels against the Jupyter messaging protocol
/// and produces a conformance report showing which protocol features each
/// kernel supports.
#[derive(Parser)]
#[command(name = "jupyter-kernel-test")]
#[command(version)]
struct Cli {
    /// Kernel names to test (as they appear in `jupyter kernelspec list`).
    /// If omitted, tests all installed kernels.
    #[arg()]
    kernels: Vec<String>,

    /// Only run tests from these tiers (1=basic, 2=interactive, 3=rich output, 4=advanced).
    /// Can be specified multiple times. If omitted, runs all tiers.
    #[arg(short, long)]
    tier: Vec<u8>,

    /// Output format: "terminal" for human-readable, "json" for machine-readable,
    /// "markdown" for GitHub-friendly tables.
    #[arg(short, long, default_value = "terminal")]
    format: OutputFormat,

    /// Write the report to a file instead of stdout.
    #[arg(short, long)]
    output: Option<String>,

    /// List available kernels and exit.
    #[arg(long)]
    list_kernels: bool,
}

#[derive(Clone, Debug)]
enum OutputFormat {
    Terminal,
    Json,
    Markdown,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "terminal" | "term" | "text" => Ok(Self::Terminal),
            "json" => Ok(Self::Json),
            "markdown" | "md" => Ok(Self::Markdown),
            _ => Err(format!(
                "unknown format '{s}': expected terminal, json, or markdown"
            )),
        }
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // List all installed kernels
    let all_kernelspecs = runtimelib::list_kernelspecs().await;

    if cli.list_kernels {
        if all_kernelspecs.is_empty() {
            eprintln!("No Jupyter kernels found. Install a kernel (e.g. `pip install ipykernel`).");
        } else {
            println!("Installed Jupyter kernels:\n");
            for ks in &all_kernelspecs {
                println!(
                    "  {:<20} {} ({})",
                    ks.kernel_name, ks.kernelspec.display_name, ks.kernelspec.language
                );
            }
        }
        return;
    }

    // Determine which kernels to test
    let kernelspecs: Vec<_> = if cli.kernels.is_empty() {
        all_kernelspecs
    } else {
        let mut selected = Vec::new();
        for name in &cli.kernels {
            match all_kernelspecs.iter().find(|k| k.kernel_name == *name) {
                Some(ks) => selected.push(ks.clone()),
                None => {
                    eprintln!("Warning: kernel '{}' not found, skipping", name);
                }
            }
        }
        selected
    };

    if kernelspecs.is_empty() {
        eprintln!("No kernels to test. Use --list-kernels to see available kernels.");
        std::process::exit(1);
    }

    // Determine which tests to run
    let test_suite = if cli.tier.is_empty() {
        tests::all_tests()
    } else {
        tests::tests_for_tiers(&cli.tier)
    };

    if test_suite.is_empty() {
        eprintln!("No tests match the specified tiers.");
        std::process::exit(1);
    }

    eprintln!(
        "Running {} tests against {} kernel(s)\n",
        test_suite.len(),
        kernelspecs.len()
    );

    let mut reports = Vec::new();

    for kernelspec in &kernelspecs {
        match run_conformance_suite(kernelspec, &test_suite).await {
            Ok(report) => {
                // Print terminal summary as tests complete
                if matches!(cli.format, OutputFormat::Terminal) {
                    eprint!("{}", report::render_terminal_summary(&report));
                }
                reports.push(report);
            }
            Err(e) => {
                eprintln!(
                    "Failed to test kernel '{}': {e}",
                    kernelspec.kernel_name
                );
            }
        }
    }

    if reports.is_empty() {
        eprintln!("\nNo kernels were successfully tested.");
        std::process::exit(1);
    }

    // Generate output
    let matrix = ConformanceMatrix {
        reports,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    let output = match cli.format {
        OutputFormat::Terminal => {
            // Already printed summaries above during the run
            String::new()
        }
        OutputFormat::Json => serde_json::to_string_pretty(&matrix).unwrap_or_default(),
        OutputFormat::Markdown => report::render_matrix_markdown(&matrix),
    };

    if !output.is_empty() {
        if let Some(path) = &cli.output {
            if let Err(e) = std::fs::write(path, &output) {
                eprintln!("Failed to write output to {path}: {e}");
                std::process::exit(1);
            }
            eprintln!("Report written to {path}");
        } else {
            println!("{output}");
        }
    }

    // Exit with error code if any test failed
    let all_passed = matrix
        .reports
        .iter()
        .all(|r| r.pass_count() == r.total_count());

    if !all_passed {
        std::process::exit(1);
    }
}
