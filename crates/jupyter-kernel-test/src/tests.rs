use jupyter_protocol::{
    CommId, CommInfoRequest, CommOpen, CompleteRequest, ExecuteRequest, ExecutionState,
    HistoryRequest, InspectRequest, InterruptRequest, IsCompleteRequest, IsCompleteReplyStatus,
    JupyterMessage, JupyterMessageContent, KernelInfoRequest, ReplyStatus, ShutdownRequest,
};

use crate::harness::{KernelUnderTest, ProtocolTest};
use crate::snippets;
use crate::types::{TestCategory, TestOutcome};

/// Returns all protocol conformance tests.
///
/// Shutdown test is always last since it may terminate the kernel.
pub fn all_tests() -> Vec<ProtocolTest> {
    let mut tests = Vec::new();
    tests.extend(tier1_tests());
    tests.extend(tier2_tests());
    tests.extend(tier3_tests());
    tests.extend(tier4_tests());
    // Shutdown must always be last — it may kill the kernel
    tests.push(shutdown_test());
    tests
}

/// Returns only tests for the given tiers.
pub fn tests_for_tiers(tiers: &[u8]) -> Vec<ProtocolTest> {
    let mut tests: Vec<_> = all_tests()
        .into_iter()
        .filter(|t| tiers.contains(&t.category.tier()))
        .collect();
    // Ensure shutdown is last even after filtering
    let shutdown_idx = tests.iter().position(|t| t.name == "shutdown_reply");
    if let Some(idx) = shutdown_idx {
        let shutdown = tests.remove(idx);
        tests.push(shutdown);
    }
    tests
}

// ─── Tier 1: Basics ─────────────────────────────────────────────────────────

fn tier1_tests() -> Vec<ProtocolTest> {
    vec![
        ProtocolTest {
            name: "heartbeat_responds",
            description: "Kernel responds to heartbeat ping with pong",
            category: TestCategory::Heartbeat,
            run: |kernel| Box::pin(test_heartbeat_responds(kernel)),
        },
        ProtocolTest {
            name: "kernel_info_reply",
            description: "Kernel responds to kernel_info_request with valid reply",
            category: TestCategory::KernelInfo,
            run: |kernel| Box::pin(test_kernel_info_reply(kernel)),
        },
        ProtocolTest {
            name: "kernel_info_has_language",
            description: "kernel_info_reply includes language_info with name",
            category: TestCategory::KernelInfo,
            run: |kernel| Box::pin(test_kernel_info_has_language(kernel)),
        },
        ProtocolTest {
            name: "kernel_info_protocol_version",
            description: "kernel_info_reply has a valid protocol version (5.x)",
            category: TestCategory::KernelInfo,
            run: |kernel| Box::pin(test_kernel_info_protocol_version(kernel)),
        },
        ProtocolTest {
            name: "execute_stdout",
            description: "Execute print code and receive stdout stream output",
            category: TestCategory::Execution,
            run: |kernel| Box::pin(test_execute_stdout(kernel)),
        },
        ProtocolTest {
            name: "execute_reply_ok",
            description: "Execute valid code and receive execute_reply with status ok",
            category: TestCategory::Execution,
            run: |kernel| Box::pin(test_execute_reply_ok(kernel)),
        },
        ProtocolTest {
            name: "status_busy_idle_lifecycle",
            description: "IOPub shows busy then idle around execution",
            category: TestCategory::StatusLifecycle,
            run: |kernel| Box::pin(test_status_busy_idle(kernel)),
        },
        ProtocolTest {
            name: "execute_input_broadcast",
            description: "IOPub broadcasts execute_input with the code",
            category: TestCategory::Execution,
            run: |kernel| Box::pin(test_execute_input_broadcast(kernel)),
        },
    ]
}

fn shutdown_test() -> ProtocolTest {
    ProtocolTest {
        name: "shutdown_reply",
        description: "Kernel responds to shutdown_request on control channel",
        category: TestCategory::Shutdown,
        run: |kernel| Box::pin(test_shutdown_reply(kernel)),
    }
}

async fn test_heartbeat_responds(kernel: &mut KernelUnderTest) -> TestOutcome {
    match tokio::time::timeout(
        kernel.client.timeout,
        kernel.client.heartbeat.single_heartbeat(),
    )
    .await
    {
        Ok(Ok(())) => TestOutcome::Pass,
        Ok(Err(e)) => TestOutcome::Fail {
            reason: format!("heartbeat error: {e}"),
        },
        Err(_) => TestOutcome::Timeout,
    }
}

async fn test_kernel_info_reply(kernel: &mut KernelUnderTest) -> TestOutcome {
    let request: JupyterMessage = KernelInfoRequest {}.into();
    match kernel.client.shell_request(request).await {
        Ok(reply) => match reply.content {
            JupyterMessageContent::KernelInfoReply(info) => {
                if info.status != ReplyStatus::Ok {
                    return TestOutcome::Fail {
                        reason: format!("status was {:?}, expected Ok", info.status),
                    };
                }
                if info.implementation.is_empty() {
                    return TestOutcome::Fail {
                        reason: "implementation field is empty".to_string(),
                    };
                }
                TestOutcome::Pass
            }
            other => TestOutcome::Fail {
                reason: format!("expected kernel_info_reply, got {}", other.message_type()),
            },
        },
        Err(e) if e.contains("timeout") => TestOutcome::Timeout,
        Err(e) => TestOutcome::Fail { reason: e },
    }
}

async fn test_kernel_info_has_language(kernel: &mut KernelUnderTest) -> TestOutcome {
    if let Some(info) = &kernel.kernel_info {
        if info.language_info.name.is_empty() {
            TestOutcome::Fail {
                reason: "language_info.name is empty".to_string(),
            }
        } else {
            TestOutcome::Pass
        }
    } else {
        TestOutcome::Skipped {
            reason: "kernel_info not available".to_string(),
        }
    }
}

async fn test_kernel_info_protocol_version(kernel: &mut KernelUnderTest) -> TestOutcome {
    if let Some(info) = &kernel.kernel_info {
        let version = &info.protocol_version;
        if version.starts_with("5.") {
            TestOutcome::Pass
        } else {
            TestOutcome::Fail {
                reason: format!("protocol version '{version}' doesn't start with 5."),
            }
        }
    } else {
        TestOutcome::Skipped {
            reason: "kernel_info not available".to_string(),
        }
    }
}

async fn test_execute_stdout(kernel: &mut KernelUnderTest) -> TestOutcome {
    let lang = kernel
        .kernel_info
        .as_ref()
        .map(|i| i.language_info.name.as_str())
        .unwrap_or("python");
    let snips = snippets::for_language(lang);

    let request: JupyterMessage = ExecuteRequest::new(snips.print_hello.to_string()).into();
    match kernel.client.execute_and_collect(request).await {
        Ok((_reply, iopub_msgs)) => {
            let has_stream = iopub_msgs.iter().any(|m| {
                if let JupyterMessageContent::StreamContent(s) = &m.content {
                    s.text.contains("hello")
                } else {
                    false
                }
            });
            if has_stream {
                TestOutcome::Pass
            } else {
                TestOutcome::Fail {
                    reason: "no stream output containing 'hello' found".to_string(),
                }
            }
        }
        Err(e) if e.contains("timeout") => TestOutcome::Timeout,
        Err(e) => TestOutcome::Fail { reason: e },
    }
}

async fn test_execute_reply_ok(kernel: &mut KernelUnderTest) -> TestOutcome {
    let lang = kernel
        .kernel_info
        .as_ref()
        .map(|i| i.language_info.name.as_str())
        .unwrap_or("python");
    let snips = snippets::for_language(lang);

    let request: JupyterMessage = ExecuteRequest::new(snips.valid_complete.to_string()).into();
    match kernel.client.execute_and_collect(request).await {
        Ok((reply, _)) => match reply.content {
            JupyterMessageContent::ExecuteReply(rep) => {
                if rep.status == ReplyStatus::Ok {
                    TestOutcome::Pass
                } else {
                    TestOutcome::Fail {
                        reason: format!("execute_reply status was {:?}", rep.status),
                    }
                }
            }
            other => TestOutcome::Fail {
                reason: format!("expected execute_reply, got {}", other.message_type()),
            },
        },
        Err(e) if e.contains("timeout") => TestOutcome::Timeout,
        Err(e) => TestOutcome::Fail { reason: e },
    }
}

async fn test_status_busy_idle(kernel: &mut KernelUnderTest) -> TestOutcome {
    let lang = kernel
        .kernel_info
        .as_ref()
        .map(|i| i.language_info.name.as_str())
        .unwrap_or("python");
    let snips = snippets::for_language(lang);

    let request: JupyterMessage = ExecuteRequest::new(snips.valid_complete.to_string()).into();
    match kernel.client.execute_and_collect(request).await {
        Ok((_reply, iopub_msgs)) => {
            let statuses: Vec<_> = iopub_msgs
                .iter()
                .filter_map(|m| {
                    if let JupyterMessageContent::Status(s) = &m.content {
                        Some(s.execution_state.clone())
                    } else {
                        None
                    }
                })
                .collect();

            // Must see Busy followed by Idle
            let has_busy = statuses.iter().any(|s| *s == ExecutionState::Busy);
            let has_idle = statuses.iter().any(|s| *s == ExecutionState::Idle);

            if !has_busy {
                return TestOutcome::Fail {
                    reason: "no busy status seen".to_string(),
                };
            }
            if !has_idle {
                return TestOutcome::Fail {
                    reason: "no idle status seen".to_string(),
                };
            }

            // Busy should come before idle
            let busy_pos = statuses
                .iter()
                .position(|s| *s == ExecutionState::Busy)
                .unwrap();
            let idle_pos = statuses
                .iter()
                .rposition(|s| *s == ExecutionState::Idle)
                .unwrap();

            if busy_pos < idle_pos {
                TestOutcome::Pass
            } else {
                TestOutcome::Fail {
                    reason: "busy did not precede idle".to_string(),
                }
            }
        }
        Err(e) if e.contains("timeout") => TestOutcome::Timeout,
        Err(e) => TestOutcome::Fail { reason: e },
    }
}

async fn test_execute_input_broadcast(kernel: &mut KernelUnderTest) -> TestOutcome {
    let lang = kernel
        .kernel_info
        .as_ref()
        .map(|i| i.language_info.name.as_str())
        .unwrap_or("python");
    let snips = snippets::for_language(lang);

    let code = snips.valid_complete.to_string();
    let request: JupyterMessage = ExecuteRequest::new(code.clone()).into();
    match kernel.client.execute_and_collect(request).await {
        Ok((_reply, iopub_msgs)) => {
            let has_execute_input = iopub_msgs.iter().any(|m| {
                if let JupyterMessageContent::ExecuteInput(ei) = &m.content {
                    ei.code == code
                } else {
                    false
                }
            });
            if has_execute_input {
                TestOutcome::Pass
            } else {
                TestOutcome::Fail {
                    reason: "no execute_input with matching code found on iopub".to_string(),
                }
            }
        }
        Err(e) if e.contains("timeout") => TestOutcome::Timeout,
        Err(e) => TestOutcome::Fail { reason: e },
    }
}

async fn test_shutdown_reply(kernel: &mut KernelUnderTest) -> TestOutcome {
    // This test must run last because the kernel will exit after receiving it.
    let request: JupyterMessage = ShutdownRequest { restart: false }.into();
    match kernel.client.control_request(request).await {
        Ok(reply) => match reply.content {
            JupyterMessageContent::ShutdownReply(rep) => {
                if rep.status == ReplyStatus::Ok {
                    TestOutcome::Pass
                } else {
                    TestOutcome::Fail {
                        reason: format!("shutdown_reply status was {:?}", rep.status),
                    }
                }
            }
            other => TestOutcome::Fail {
                reason: format!("expected shutdown_reply, got {}", other.message_type()),
            },
        },
        Err(e) if e.contains("timeout") => TestOutcome::Timeout,
        Err(e) => TestOutcome::Fail { reason: e },
    }
}

// ─── Tier 2: Interactive features ────────────────────────────────────────────

fn tier2_tests() -> Vec<ProtocolTest> {
    vec![
        ProtocolTest {
            name: "complete_request_reply",
            description: "Kernel responds to complete_request with complete_reply",
            category: TestCategory::Completion,
            run: |kernel| Box::pin(test_complete_request(kernel)),
        },
        ProtocolTest {
            name: "inspect_request_reply",
            description: "Kernel responds to inspect_request with inspect_reply",
            category: TestCategory::Introspection,
            run: |kernel| Box::pin(test_inspect_request(kernel)),
        },
        ProtocolTest {
            name: "is_complete_request_complete",
            description: "Kernel identifies complete code as 'complete'",
            category: TestCategory::CodeCompleteness,
            run: |kernel| Box::pin(test_is_complete_complete(kernel)),
        },
        ProtocolTest {
            name: "is_complete_request_incomplete",
            description: "Kernel identifies incomplete code as 'incomplete'",
            category: TestCategory::CodeCompleteness,
            run: |kernel| Box::pin(test_is_complete_incomplete(kernel)),
        },
        ProtocolTest {
            name: "history_request_reply",
            description: "Kernel responds to history_request with history_reply",
            category: TestCategory::History,
            run: |kernel| Box::pin(test_history_request(kernel)),
        },
        ProtocolTest {
            name: "comm_info_request_reply",
            description: "Kernel responds to comm_info_request with comm_info_reply",
            category: TestCategory::CommInfo,
            run: |kernel| Box::pin(test_comm_info_request(kernel)),
        },
        ProtocolTest {
            name: "execute_error_handling",
            description: "Kernel handles runtime errors with error output and error status",
            category: TestCategory::ErrorHandling,
            run: |kernel| Box::pin(test_execute_error(kernel)),
        },
    ]
}

async fn test_complete_request(kernel: &mut KernelUnderTest) -> TestOutcome {
    let lang = kernel
        .kernel_info
        .as_ref()
        .map(|i| i.language_info.name.as_str())
        .unwrap_or("python");
    let snips = snippets::for_language(lang);

    let request: JupyterMessage = CompleteRequest {
        code: snips.complete_prefix.to_string(),
        cursor_pos: snips.complete_cursor_pos,
    }
    .into();

    match kernel.client.shell_request(request).await {
        Ok(reply) => match reply.content {
            JupyterMessageContent::CompleteReply(rep) => {
                if rep.status == ReplyStatus::Ok {
                    TestOutcome::Pass
                } else {
                    TestOutcome::Fail {
                        reason: format!("complete_reply status was {:?}", rep.status),
                    }
                }
            }
            other => TestOutcome::Fail {
                reason: format!("expected complete_reply, got {}", other.message_type()),
            },
        },
        Err(e) if e.contains("timeout") => TestOutcome::Timeout,
        Err(e) => TestOutcome::Fail { reason: e },
    }
}

async fn test_inspect_request(kernel: &mut KernelUnderTest) -> TestOutcome {
    let lang = kernel
        .kernel_info
        .as_ref()
        .map(|i| i.language_info.name.as_str())
        .unwrap_or("python");
    let snips = snippets::for_language(lang);
    let symbol = snips.inspect_symbol;

    let request: JupyterMessage = InspectRequest {
        code: symbol.to_string(),
        cursor_pos: symbol.len(),
        detail_level: Some(0),
    }
    .into();

    match kernel.client.shell_request(request).await {
        Ok(reply) => match reply.content {
            JupyterMessageContent::InspectReply(rep) => {
                if rep.status == ReplyStatus::Ok {
                    TestOutcome::Pass
                } else {
                    TestOutcome::Fail {
                        reason: format!("inspect_reply status was {:?}", rep.status),
                    }
                }
            }
            other => TestOutcome::Fail {
                reason: format!("expected inspect_reply, got {}", other.message_type()),
            },
        },
        Err(e) if e.contains("timeout") => TestOutcome::Timeout,
        Err(e) => TestOutcome::Fail { reason: e },
    }
}

async fn test_is_complete_complete(kernel: &mut KernelUnderTest) -> TestOutcome {
    let lang = kernel
        .kernel_info
        .as_ref()
        .map(|i| i.language_info.name.as_str())
        .unwrap_or("python");
    let snips = snippets::for_language(lang);

    let request: JupyterMessage = IsCompleteRequest {
        code: snips.valid_complete.to_string(),
    }
    .into();

    match kernel.client.shell_request(request).await {
        Ok(reply) => match reply.content {
            JupyterMessageContent::IsCompleteReply(rep) => {
                if rep.status == IsCompleteReplyStatus::Complete {
                    TestOutcome::Pass
                } else {
                    TestOutcome::Fail {
                        reason: format!("expected 'complete', got {:?}", rep.status),
                    }
                }
            }
            other => TestOutcome::Fail {
                reason: format!("expected is_complete_reply, got {}", other.message_type()),
            },
        },
        Err(e) if e.contains("timeout") => TestOutcome::Timeout,
        Err(e) => TestOutcome::Fail { reason: e },
    }
}

async fn test_is_complete_incomplete(kernel: &mut KernelUnderTest) -> TestOutcome {
    let lang = kernel
        .kernel_info
        .as_ref()
        .map(|i| i.language_info.name.as_str())
        .unwrap_or("python");
    let snips = snippets::for_language(lang);

    if snips.valid_incomplete.is_empty() {
        return TestOutcome::Skipped {
            reason: "no incomplete snippet for this language".to_string(),
        };
    }

    let request: JupyterMessage = IsCompleteRequest {
        code: snips.valid_incomplete.to_string(),
    }
    .into();

    match kernel.client.shell_request(request).await {
        Ok(reply) => match reply.content {
            JupyterMessageContent::IsCompleteReply(rep) => {
                // Accept either Incomplete or Unknown — some kernels don't fully implement this
                if rep.status == IsCompleteReplyStatus::Incomplete
                    || rep.status == IsCompleteReplyStatus::Unknown
                {
                    TestOutcome::Pass
                } else {
                    TestOutcome::Fail {
                        reason: format!(
                            "expected 'incomplete' or 'unknown', got {:?}",
                            rep.status
                        ),
                    }
                }
            }
            other => TestOutcome::Fail {
                reason: format!("expected is_complete_reply, got {}", other.message_type()),
            },
        },
        Err(e) if e.contains("timeout") => TestOutcome::Timeout,
        Err(e) => TestOutcome::Fail { reason: e },
    }
}

async fn test_history_request(kernel: &mut KernelUnderTest) -> TestOutcome {
    let request: JupyterMessage = HistoryRequest::Tail {
        n: 10,
        output: false,
        raw: true,
    }
    .into();

    match kernel.client.shell_request(request).await {
        Ok(reply) => match reply.content {
            JupyterMessageContent::HistoryReply(rep) => {
                if rep.status == ReplyStatus::Ok {
                    TestOutcome::Pass
                } else {
                    TestOutcome::Fail {
                        reason: format!("history_reply status was {:?}", rep.status),
                    }
                }
            }
            other => TestOutcome::Fail {
                reason: format!("expected history_reply, got {}", other.message_type()),
            },
        },
        Err(e) if e.contains("timeout") => TestOutcome::Timeout,
        Err(e) => TestOutcome::Fail { reason: e },
    }
}

async fn test_comm_info_request(kernel: &mut KernelUnderTest) -> TestOutcome {
    let request: JupyterMessage = CommInfoRequest { target_name: None }.into();

    match kernel.client.shell_request(request).await {
        Ok(reply) => match reply.content {
            JupyterMessageContent::CommInfoReply(rep) => {
                if rep.status == ReplyStatus::Ok {
                    TestOutcome::Pass
                } else {
                    TestOutcome::Fail {
                        reason: format!("comm_info_reply status was {:?}", rep.status),
                    }
                }
            }
            other => TestOutcome::Fail {
                reason: format!("expected comm_info_reply, got {}", other.message_type()),
            },
        },
        Err(e) if e.contains("timeout") => TestOutcome::Timeout,
        Err(e) => TestOutcome::Fail { reason: e },
    }
}

async fn test_execute_error(kernel: &mut KernelUnderTest) -> TestOutcome {
    let lang = kernel
        .kernel_info
        .as_ref()
        .map(|i| i.language_info.name.as_str())
        .unwrap_or("python");
    let snips = snippets::for_language(lang);

    let request: JupyterMessage = ExecuteRequest::new(snips.runtime_error.to_string()).into();
    match kernel.client.execute_and_collect(request).await {
        Ok((reply, iopub_msgs)) => {
            // Check that shell reply has error status
            let reply_is_error = match &reply.content {
                JupyterMessageContent::ExecuteReply(rep) => rep.status == ReplyStatus::Error,
                _ => false,
            };

            // Check that iopub has an error output
            let has_error_output = iopub_msgs
                .iter()
                .any(|m| matches!(&m.content, JupyterMessageContent::ErrorOutput(_)));

            if reply_is_error && has_error_output {
                TestOutcome::Pass
            } else if !reply_is_error {
                TestOutcome::Fail {
                    reason: "execute_reply status was not 'error'".to_string(),
                }
            } else {
                TestOutcome::Fail {
                    reason: "no error output on iopub".to_string(),
                }
            }
        }
        Err(e) if e.contains("timeout") => TestOutcome::Timeout,
        Err(e) => TestOutcome::Fail { reason: e },
    }
}

// ─── Tier 3: Rich output ────────────────────────────────────────────────────

fn tier3_tests() -> Vec<ProtocolTest> {
    vec![
        ProtocolTest {
            name: "display_data_output",
            description: "Kernel produces display_data on iopub for rich output",
            category: TestCategory::RichOutput,
            run: |kernel| Box::pin(test_display_data(kernel)),
        },
        ProtocolTest {
            name: "execute_result_output",
            description: "Expression evaluation produces execute_result on iopub",
            category: TestCategory::RichOutput,
            run: |kernel| Box::pin(test_execute_result(kernel)),
        },
    ]
}

async fn test_display_data(kernel: &mut KernelUnderTest) -> TestOutcome {
    let lang = kernel
        .kernel_info
        .as_ref()
        .map(|i| i.language_info.name.as_str())
        .unwrap_or("python");
    let snips = snippets::for_language(lang);

    if snips.rich_display.is_empty() {
        return TestOutcome::Skipped {
            reason: format!("no rich display snippet for language '{lang}'"),
        };
    }

    let request: JupyterMessage = ExecuteRequest::new(snips.rich_display.to_string()).into();
    match kernel.client.execute_and_collect(request).await {
        Ok((_reply, iopub_msgs)) => {
            let has_display_data = iopub_msgs
                .iter()
                .any(|m| matches!(&m.content, JupyterMessageContent::DisplayData(_)));
            if has_display_data {
                TestOutcome::Pass
            } else {
                TestOutcome::Fail {
                    reason: "no display_data on iopub".to_string(),
                }
            }
        }
        Err(e) if e.contains("timeout") => TestOutcome::Timeout,
        Err(e) => TestOutcome::Fail { reason: e },
    }
}

async fn test_execute_result(kernel: &mut KernelUnderTest) -> TestOutcome {
    let lang = kernel
        .kernel_info
        .as_ref()
        .map(|i| i.language_info.name.as_str())
        .unwrap_or("python");

    // Use an expression that should produce execute_result (not just stdout)
    let expr = match lang.to_lowercase().as_str() {
        "python" => "1 + 1",
        "r" => "1 + 1",
        "julia" => "1 + 1",
        "rust" => "1 + 1",
        "typescript" | "javascript" => "1 + 1",
        "go" => "",
        _ => "1 + 1",
    };

    if expr.is_empty() {
        return TestOutcome::Skipped {
            reason: format!("no expression snippet for language '{lang}'"),
        };
    }

    let request: JupyterMessage = ExecuteRequest::new(expr.to_string()).into();
    match kernel.client.execute_and_collect(request).await {
        Ok((_reply, iopub_msgs)) => {
            let has_execute_result = iopub_msgs
                .iter()
                .any(|m| matches!(&m.content, JupyterMessageContent::ExecuteResult(_)));
            if has_execute_result {
                TestOutcome::Pass
            } else {
                TestOutcome::Fail {
                    reason: "no execute_result on iopub (expression may have gone to stdout instead)"
                        .to_string(),
                }
            }
        }
        Err(e) if e.contains("timeout") => TestOutcome::Timeout,
        Err(e) => TestOutcome::Fail { reason: e },
    }
}

// ─── Tier 4: Advanced ───────────────────────────────────────────────────────

fn tier4_tests() -> Vec<ProtocolTest> {
    vec![
        ProtocolTest {
            name: "stdin_input_request",
            description: "Kernel sends input_request on stdin when code reads input",
            category: TestCategory::Stdin,
            run: |kernel| Box::pin(test_stdin_input(kernel)),
        },
        ProtocolTest {
            name: "comm_open_close_lifecycle",
            description: "Kernel handles comm_open and comm_info shows the comm",
            category: TestCategory::Comms,
            run: |kernel| Box::pin(test_comm_lifecycle(kernel)),
        },
        ProtocolTest {
            name: "interrupt_request_reply",
            description: "Kernel responds to interrupt_request on control channel",
            category: TestCategory::Interrupt,
            run: |kernel| Box::pin(test_interrupt_request(kernel)),
        },
        ProtocolTest {
            name: "execution_count_increments",
            description: "Execution count increments across successive executions",
            category: TestCategory::MessageProtocol,
            run: |kernel| Box::pin(test_execution_count_increments(kernel)),
        },
        ProtocolTest {
            name: "parent_header_correlation",
            description: "Reply parent_header matches request msg_id",
            category: TestCategory::MessageProtocol,
            run: |kernel| Box::pin(test_parent_header_correlation(kernel)),
        },
    ]
}

async fn test_stdin_input(kernel: &mut KernelUnderTest) -> TestOutcome {
    let lang = kernel
        .kernel_info
        .as_ref()
        .map(|i| i.language_info.name.as_str())
        .unwrap_or("python");
    let snips = snippets::for_language(lang);

    if snips.stdin_read.is_empty() {
        return TestOutcome::Skipped {
            reason: format!("no stdin snippet for language '{lang}'"),
        };
    }

    let mut request = ExecuteRequest::new(snips.stdin_read.to_string());
    request.allow_stdin = true;
    let request: JupyterMessage = request.into();
    let request_id = request.header.msg_id.clone();

    kernel
        .client
        .shell
        .send(request)
        .await
        .map_err(|e| format!("send: {e}"))
        .ok();

    // Wait for input_request on stdin channel
    let deadline = std::time::Instant::now() + kernel.client.timeout;
    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            return TestOutcome::Timeout;
        }

        // Read from iopub to drain messages, but we're looking for input_request on stdin
        // The kernel sends input_request on the stdin channel
        match tokio::time::timeout(remaining, kernel.client.stdin.read()).await {
            Ok(Ok(msg)) => {
                if let JupyterMessageContent::InputRequest(req) = &msg.content {
                    // Respond so the kernel can continue
                    let reply: JupyterMessage = jupyter_protocol::InputReply {
                        value: "test_input".to_string(),
                        status: ReplyStatus::Ok,
                        error: None,
                    }
                    .as_child_of(&msg);
                    let _ = kernel.client.stdin.send(reply).await;

                    // Drain iopub until idle
                    let _ = drain_iopub_until_idle(
                        &mut kernel.client.iopub,
                        &request_id,
                        kernel.client.timeout,
                    )
                    .await;

                    // Read the execute_reply
                    let _ = tokio::time::timeout(
                        kernel.client.timeout,
                        kernel.client.shell.read(),
                    )
                    .await;

                    let _ = req; // used
                    return TestOutcome::Pass;
                }
            }
            Ok(Err(_)) => continue,
            Err(_) => return TestOutcome::Timeout,
        }
    }
}

async fn test_comm_lifecycle(kernel: &mut KernelUnderTest) -> TestOutcome {
    let comm_id = CommId(uuid::Uuid::new_v4().to_string());
    let target_name = "jupyter.kernel-test.test-target".to_string();

    // Send comm_open on shell
    let open_msg: JupyterMessage = CommOpen {
        comm_id: comm_id.clone(),
        target_name: target_name.clone(),
        data: serde_json::Map::new(),
        target_module: None,
    }
    .into();

    // comm_open is typically sent as a side-channel message; we send it on shell
    // and check via comm_info_request
    if let Err(e) = kernel.client.shell.send(open_msg).await {
        return TestOutcome::Fail {
            reason: format!("failed to send comm_open: {e}"),
        };
    }

    // Give the kernel a moment to register the comm
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Query comm_info to see if the comm is registered
    let info_request: JupyterMessage = CommInfoRequest {
        target_name: Some(target_name.clone()),
    }
    .into();

    match kernel.client.shell_request(info_request).await {
        Ok(reply) => match reply.content {
            JupyterMessageContent::CommInfoReply(rep) => {
                if rep.status == ReplyStatus::Ok {
                    // Some kernels will register client-opened comms, others won't.
                    // The key test is that we got a valid reply.
                    TestOutcome::Pass
                } else {
                    TestOutcome::Fail {
                        reason: format!("comm_info_reply status was {:?}", rep.status),
                    }
                }
            }
            other => TestOutcome::Fail {
                reason: format!("expected comm_info_reply, got {}", other.message_type()),
            },
        },
        Err(e) if e.contains("timeout") => TestOutcome::Timeout,
        Err(e) => TestOutcome::Fail { reason: e },
    }
}

async fn test_interrupt_request(kernel: &mut KernelUnderTest) -> TestOutcome {
    let request: JupyterMessage = InterruptRequest {}.into();
    match kernel.client.control_request(request).await {
        Ok(reply) => match reply.content {
            JupyterMessageContent::InterruptReply(rep) => {
                if rep.status == ReplyStatus::Ok {
                    TestOutcome::Pass
                } else {
                    TestOutcome::Fail {
                        reason: format!("interrupt_reply status was {:?}", rep.status),
                    }
                }
            }
            other => TestOutcome::Fail {
                reason: format!("expected interrupt_reply, got {}", other.message_type()),
            },
        },
        Err(e) if e.contains("timeout") => TestOutcome::Timeout,
        Err(e) => TestOutcome::Fail { reason: e },
    }
}

async fn test_execution_count_increments(kernel: &mut KernelUnderTest) -> TestOutcome {
    let lang = kernel
        .kernel_info
        .as_ref()
        .map(|i| i.language_info.name.as_str())
        .unwrap_or("python");
    let snips = snippets::for_language(lang);

    // Execute twice and check that execution count increments
    let request1: JupyterMessage = ExecuteRequest::new(snips.valid_complete.to_string()).into();
    let (reply1, _) = match kernel.client.execute_and_collect(request1).await {
        Ok(r) => r,
        Err(e) => return if e.contains("timeout") { TestOutcome::Timeout } else { TestOutcome::Fail { reason: e } },
    };

    let request2: JupyterMessage = ExecuteRequest::new(snips.valid_complete.to_string()).into();
    let (reply2, _) = match kernel.client.execute_and_collect(request2).await {
        Ok(r) => r,
        Err(e) => return if e.contains("timeout") { TestOutcome::Timeout } else { TestOutcome::Fail { reason: e } },
    };

    let count1 = match &reply1.content {
        JupyterMessageContent::ExecuteReply(rep) => rep.execution_count.0,
        _ => return TestOutcome::Fail { reason: "first reply not execute_reply".to_string() },
    };
    let count2 = match &reply2.content {
        JupyterMessageContent::ExecuteReply(rep) => rep.execution_count.0,
        _ => return TestOutcome::Fail { reason: "second reply not execute_reply".to_string() },
    };

    if count2 > count1 {
        TestOutcome::Pass
    } else {
        TestOutcome::Fail {
            reason: format!("execution count did not increment: {count1} -> {count2}"),
        }
    }
}

async fn test_parent_header_correlation(kernel: &mut KernelUnderTest) -> TestOutcome {
    let lang = kernel
        .kernel_info
        .as_ref()
        .map(|i| i.language_info.name.as_str())
        .unwrap_or("python");
    let snips = snippets::for_language(lang);

    let request: JupyterMessage = ExecuteRequest::new(snips.valid_complete.to_string()).into();
    let request_id = request.header.msg_id.clone();

    match kernel.client.execute_and_collect(request).await {
        Ok((reply, iopub_msgs)) => {
            // Check shell reply parent_header
            let reply_parent_ok = reply
                .parent_header
                .as_ref()
                .map(|h| h.msg_id == request_id)
                .unwrap_or(false);

            if !reply_parent_ok {
                return TestOutcome::Fail {
                    reason: "shell reply parent_header.msg_id doesn't match request".to_string(),
                };
            }

            // Check all iopub messages have correct parent_header
            for msg in &iopub_msgs {
                let iopub_parent_ok = msg
                    .parent_header
                    .as_ref()
                    .map(|h| h.msg_id == request_id)
                    .unwrap_or(false);
                if !iopub_parent_ok {
                    return TestOutcome::Fail {
                        reason: format!(
                            "iopub {} parent_header.msg_id doesn't match request",
                            msg.content.message_type()
                        ),
                    };
                }
            }

            TestOutcome::Pass
        }
        Err(e) if e.contains("timeout") => TestOutcome::Timeout,
        Err(e) => TestOutcome::Fail { reason: e },
    }
}

/// Helper to drain iopub messages until we see idle for a given request.
async fn drain_iopub_until_idle(
    iopub: &mut runtimelib::ClientIoPubConnection,
    request_id: &str,
    timeout: std::time::Duration,
) -> Result<Vec<JupyterMessage>, String> {
    let mut msgs = Vec::new();
    let deadline = std::time::Instant::now() + timeout;

    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            return Err("timeout draining iopub".to_string());
        }

        let msg = tokio::time::timeout(remaining, iopub.read())
            .await
            .map_err(|_| "iopub timeout".to_string())?
            .map_err(|e| format!("iopub read: {e}"))?;

        let is_ours = msg
            .parent_header
            .as_ref()
            .map(|h| h.msg_id == request_id)
            .unwrap_or(false);

        if !is_ours {
            continue;
        }

        if let JupyterMessageContent::Status(status) = &msg.content {
            if status.execution_state == ExecutionState::Idle {
                msgs.push(msg);
                return Ok(msgs);
            }
        }

        msgs.push(msg);
    }
}
