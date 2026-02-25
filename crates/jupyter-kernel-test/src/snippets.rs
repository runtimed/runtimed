/// Language-aware code snippets for protocol tests.
///
/// Different kernels need different syntax for the same operation
/// (e.g. printing to stdout). This module provides snippets keyed by
/// the kernel's language name so tests can exercise real behavior
/// without hardcoding Python.

/// A set of code snippets for a particular language.
pub struct LangSnippets {
    /// Code that prints "hello" to stdout (e.g. `print("hello")`)
    pub print_hello: &'static str,
    /// Code that is syntactically valid and complete
    pub valid_complete: &'static str,
    /// Code that is syntactically incomplete (e.g. an open paren)
    pub valid_incomplete: &'static str,
    /// Code that raises/throws a runtime error
    pub runtime_error: &'static str,
    /// A known symbol to use for tab-completion (partial identifier)
    pub complete_prefix: &'static str,
    /// The cursor position for the completion prefix
    pub complete_cursor_pos: usize,
    /// A known symbol to inspect (e.g. `print`)
    pub inspect_symbol: &'static str,
    /// Code that reads stdin (e.g. `input("prompt: ")`)
    pub stdin_read: &'static str,
    /// Code that produces a rich display_data output (e.g. display(HTML(...)))
    pub rich_display: &'static str,
    /// Code that runs a long-running or infinite loop (for interrupt testing)
    pub infinite_loop: &'static str,
}

/// Get language snippets for a kernel, falling back to Python-like defaults.
pub fn for_language(language: &str) -> LangSnippets {
    match language.to_lowercase().as_str() {
        "python" => LangSnippets {
            print_hello: "print('hello')",
            valid_complete: "x = 1",
            valid_incomplete: "if True:",
            runtime_error: "raise ValueError('test error')",
            complete_prefix: "pri",
            complete_cursor_pos: 3,
            inspect_symbol: "print",
            stdin_read: "input('prompt: ')",
            rich_display: "from IPython.display import display, HTML; display(HTML('<b>bold</b>'))",
            infinite_loop: "import time\nwhile True:\n    time.sleep(0.01)",
        },
        "r" => LangSnippets {
            print_hello: "cat('hello')",
            valid_complete: "x <- 1",
            valid_incomplete: "if (TRUE) {",
            runtime_error: "stop('test error')",
            complete_prefix: "pri",
            complete_cursor_pos: 3,
            inspect_symbol: "print",
            stdin_read: "readline('prompt: ')",
            rich_display: "IRdisplay::display_html('<b>bold</b>')",
            infinite_loop: "while(TRUE) { Sys.sleep(0.01) }",
        },
        "rust" => LangSnippets {
            print_hello: "println!(\"hello\");",
            valid_complete: "let x = 1;",
            valid_incomplete: "fn foo() {",
            runtime_error: "panic!(\"test error\");",
            complete_prefix: "prin",
            complete_cursor_pos: 4,
            inspect_symbol: "println",
            stdin_read: "",  // evcxr doesn't support stdin
            rich_display: "",
            infinite_loop: "loop { std::thread::sleep(std::time::Duration::from_millis(10)); }",
        },
        "julia" => LangSnippets {
            print_hello: "println(\"hello\")",
            valid_complete: "x = 1",
            valid_incomplete: "function foo()",
            runtime_error: "error(\"test error\")",
            complete_prefix: "print",
            complete_cursor_pos: 5,
            inspect_symbol: "println",
            stdin_read: "readline()",
            rich_display: "display(\"text/html\", \"<b>bold</b>\")",
            infinite_loop: "while true; sleep(0.01); end",
        },
        "typescript" | "javascript" => LangSnippets {
            print_hello: "console.log('hello')",
            valid_complete: "const x = 1;",
            valid_incomplete: "function foo() {",
            runtime_error: "throw new Error('test error');",
            complete_prefix: "cons",
            complete_cursor_pos: 4,
            inspect_symbol: "console",
            stdin_read: "",
            rich_display: "",
            infinite_loop: "while(true) {}",
        },
        "go" => LangSnippets {
            print_hello: "fmt.Println(\"hello\")",
            valid_complete: "x := 1",
            valid_incomplete: "func foo() {",
            runtime_error: "panic(\"test error\")",
            complete_prefix: "fmt.Pr",
            complete_cursor_pos: 6,
            inspect_symbol: "fmt",
            stdin_read: "",
            rich_display: "",
            infinite_loop: "for { time.Sleep(10 * time.Millisecond) }",
        },
        // Fallback: use Python-like syntax
        _ => LangSnippets {
            print_hello: "print('hello')",
            valid_complete: "x = 1",
            valid_incomplete: "if True:",
            runtime_error: "raise ValueError('test error')",
            complete_prefix: "pri",
            complete_cursor_pos: 3,
            inspect_symbol: "print",
            stdin_read: "input('prompt: ')",
            rich_display: "",
            infinite_loop: "import time\nwhile True:\n    time.sleep(0.01)",
        },
    }
}
