//! CI lint: ensure no tokio::sync::Mutex (or RwLock) guards are held across
//! .await points anywhere in runtimelib.
//!
//! Uses the async-rust-lsp rule engine (tree-sitter based) to scan all
//! runtimelib source files. Any violation is a hard CI failure.
//!
//! Holding a `tokio::sync::Mutex` or `tokio::sync::RwLock` guard across an
//! `.await` causes convoy deadlocks: if the task holding the guard is
//! suspended on an async op, every other task waiting for the lock blocks
//! indefinitely. Clippy does not catch this pattern.
//!
//! Fix style: scope the lock in its own block so the guard drops at the
//! block boundary, before the next `.await`. Do not rely on `drop(guard)`
//! calls — the lint can only see lexical scope, not drop flow.
//!
//! Modeled on `nteract/desktop`'s `crates/runtimed/tests/tokio_mutex_lint.rs`,
//! which reached zero violations on 2026-04-08 after four burndown PRs.
//!
//! Runs under the `tokio-runtime` feature because that is the feature that
//! brings the async code paths into the build in the first place — but the
//! lint itself is a pure-source scan and does not execute runtimelib code,
//! so it works under any feature set.

#[test]
fn runtimelib_has_no_tokio_mutex_across_await() {
    let src_dir = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/src"));

    let rs_files: Vec<std::path::PathBuf> = std::fs::read_dir(&src_dir)
        .unwrap_or_else(|e| panic!("failed to read src dir {}: {e}", src_dir.display()))
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.extension().is_some_and(|ext| ext == "rs") {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    assert!(
        !rs_files.is_empty(),
        "no .rs files found in {}",
        src_dir.display()
    );

    let mut violations = Vec::new();

    for path in &rs_files {
        let source = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));

        let diagnostics =
            async_rust_lsp::rules::mutex_across_await::check_mutex_across_await(&source);

        let file_name = path.file_name().map_or_else(
            || panic!("no file name for {}", path.display()),
            |n| n.to_string_lossy().to_string(),
        );

        for d in diagnostics {
            violations.push(format!(
                "  {}:{}: {}",
                file_name,
                d.range.start.line + 1,
                d.message
            ));
        }
    }

    if !violations.is_empty() {
        let mut msg = format!(
            "Found {} tokio Mutex guard(s) held across .await in runtimelib sources:\n\n",
            violations.len()
        );
        for v in &violations {
            msg.push_str(v);
            msg.push('\n');
        }
        msg.push_str(
            "\nFix: scope each lock in its own block so the guard drops before the next .await.\n",
        );
        panic!("{msg}");
    }
}
