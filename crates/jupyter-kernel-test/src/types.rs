use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::Duration;

/// Categories of protocol conformance tests, organized by protocol area.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestCategory {
    /// Heartbeat channel tests
    Heartbeat,
    /// Kernel info and lifecycle
    KernelInfo,
    /// Code execution (shell channel)
    Execution,
    /// Tab completion
    Completion,
    /// Object inspection
    Introspection,
    /// Code completeness checking
    CodeCompleteness,
    /// History access
    History,
    /// Comm info queries
    CommInfo,
    /// Error handling
    ErrorHandling,
    /// IOPub status lifecycle (busy/idle)
    StatusLifecycle,
    /// Rich display output
    RichOutput,
    /// Stdin input request/reply
    Stdin,
    /// Comm open/msg/close lifecycle
    Comms,
    /// Kernel interrupt
    Interrupt,
    /// Kernel shutdown
    Shutdown,
    /// Debug Adapter Protocol
    Debug,
    /// Message protocol correctness (parent headers, execution counts, etc.)
    MessageProtocol,
}

impl TestCategory {
    pub fn tier(&self) -> u8 {
        match self {
            Self::Heartbeat
            | Self::KernelInfo
            | Self::Execution
            | Self::StatusLifecycle
            | Self::Shutdown => 1,

            Self::Completion
            | Self::Introspection
            | Self::CodeCompleteness
            | Self::History
            | Self::CommInfo
            | Self::ErrorHandling => 2,

            Self::RichOutput => 3,

            Self::Stdin
            | Self::Comms
            | Self::Interrupt
            | Self::Debug
            | Self::MessageProtocol => 4,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Heartbeat => "Heartbeat channel",
            Self::KernelInfo => "Kernel info",
            Self::Execution => "Code execution",
            Self::Completion => "Tab completion",
            Self::Introspection => "Object inspection",
            Self::CodeCompleteness => "Code completeness",
            Self::History => "Execution history",
            Self::CommInfo => "Comm info",
            Self::ErrorHandling => "Error handling",
            Self::StatusLifecycle => "Status busy/idle lifecycle",
            Self::RichOutput => "Rich display output",
            Self::Stdin => "Stdin input",
            Self::Comms => "Comms lifecycle",
            Self::Interrupt => "Kernel interrupt",
            Self::Shutdown => "Kernel shutdown",
            Self::Debug => "Debug Adapter Protocol",
            Self::MessageProtocol => "Message protocol correctness",
        }
    }
}

/// Outcome of a single protocol conformance test.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum TestOutcome {
    /// Test passed: kernel behaves according to spec.
    Pass,
    /// Test failed: kernel responded but incorrectly.
    Fail { reason: String },
    /// Kernel did not respond within the timeout.
    Timeout,
    /// Test was skipped (e.g. kernel doesn't have the prerequisite).
    Skipped { reason: String },
}

impl TestOutcome {
    pub fn is_pass(&self) -> bool {
        matches!(self, TestOutcome::Pass)
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            TestOutcome::Pass => "PASS",
            TestOutcome::Fail { .. } => "FAIL",
            TestOutcome::Timeout => "TIMEOUT",
            TestOutcome::Skipped { .. } => "SKIP",
        }
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            TestOutcome::Pass => "✅",
            TestOutcome::Fail { .. } => "❌",
            TestOutcome::Timeout => "⏱️",
            TestOutcome::Skipped { .. } => "⏭️",
        }
    }
}

/// Result of running a single test.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    /// Unique test name (e.g. "heartbeat_responds")
    pub test_name: String,
    /// Human-readable description
    pub description: String,
    /// Which protocol area this tests
    pub category: TestCategory,
    /// Which tier (1=basic, 4=advanced)
    pub tier: u8,
    /// The outcome
    pub outcome: TestOutcome,
    /// How long the test took
    #[serde(with = "duration_millis")]
    pub duration: Duration,
}

/// Report for a single kernel's conformance run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelReport {
    /// Kernel name (from kernelspec)
    pub kernel_name: String,
    /// Kernel display name
    pub display_name: String,
    /// Kernel language
    pub language: String,
    /// Protocol version reported by the kernel (if we got kernel_info)
    pub protocol_version: Option<String>,
    /// Implementation name reported by the kernel
    pub implementation: Option<String>,
    /// Individual test results
    pub results: Vec<TestResult>,
    /// Timestamp of the run
    pub timestamp: String,
    /// Total duration of the run
    #[serde(with = "duration_millis")]
    pub total_duration: Duration,
}

impl KernelReport {
    pub fn pass_count(&self) -> usize {
        self.results.iter().filter(|r| r.outcome.is_pass()).count()
    }

    pub fn total_count(&self) -> usize {
        self.results
            .iter()
            .filter(|r| !matches!(r.outcome, TestOutcome::Skipped { .. }))
            .count()
    }

    pub fn score_by_tier(&self) -> BTreeMap<u8, (usize, usize)> {
        let mut scores: BTreeMap<u8, (usize, usize)> = BTreeMap::new();
        for result in &self.results {
            if matches!(result.outcome, TestOutcome::Skipped { .. }) {
                continue;
            }
            let entry = scores.entry(result.tier).or_insert((0, 0));
            entry.1 += 1;
            if result.outcome.is_pass() {
                entry.0 += 1;
            }
        }
        scores
    }
}

/// Aggregated report across multiple kernels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConformanceMatrix {
    pub reports: Vec<KernelReport>,
    pub timestamp: String,
}

mod duration_millis {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S: Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u64(d.as_millis() as u64)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
        let millis = u64::deserialize(d)?;
        Ok(Duration::from_millis(millis))
    }
}
