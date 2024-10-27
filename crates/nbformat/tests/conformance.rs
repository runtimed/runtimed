use nbformat::v4::{Cell, Output};
use nbformat::v4::{CellId, Notebook};
use std::fs;
use std::path::Path;

fn read_notebook(path: &str) -> String {
    fs::read_to_string(Path::new(path)).expect("Failed to read notebook file")
}

#[test]
fn test_parse_basic_notebook() {
    let notebook_json = read_notebook("tests/notebooks/test4.ipynb");
    let notebook: Notebook =
        serde_json::from_str(&notebook_json).expect("Failed to parse notebook");

    assert_eq!(notebook.nbformat, 4);
    assert_eq!(notebook.nbformat_minor, 0);
    assert_eq!(notebook.cells.len(), 9);

    assert!(notebook.metadata.kernelspec.is_none());
    assert!(notebook.metadata.language_info.is_none());

    // Check first cell (markdown)
    let first_cell = &notebook.cells[0];
    if let Cell::Markdown { source, .. } = first_cell {
        assert_eq!(source, &vec!["# nbconvert latex test"]);
    } else {
        panic!("First cell should be markdown");
    }

    // Check a code cell
    let code_cell = &notebook.cells[3];
    if let Cell::Code {
        source,
        execution_count,
        outputs,
        ..
    } = code_cell
    {
        assert_eq!(source, &vec!["print(\"hello\")"]);
        assert_eq!(*execution_count, Some(1));
        assert_eq!(outputs.len(), 1);
        if let Output::Stream { name, text } = &outputs[0] {
            assert_eq!(name, "stdout");
            assert_eq!(text.0, "hello\n");
        } else {
            panic!("Expected stream output");
        }
    } else {
        panic!("Expected code cell");
    }
}
#[test]
fn test_parse_v4_5_notebook() {
    let notebook_json = read_notebook("tests/notebooks/test4.5.ipynb");
    let notebook: Notebook =
        serde_json::from_str(&notebook_json).expect("Failed to parse notebook");

    assert_eq!(notebook.nbformat, 4);
    assert_eq!(notebook.nbformat_minor, 5);
    assert!(notebook.cells.len() > 0);

    // Check metadata
    assert!(notebook.metadata.kernelspec.is_some());
    let kernelspec = notebook.metadata.kernelspec.as_ref().unwrap();
    assert_eq!(kernelspec.name, "python3");

    assert!(notebook.metadata.language_info.is_some());
    let lang_info = notebook.metadata.language_info.as_ref().unwrap();
    assert_eq!(lang_info.name, "python");

    // Check a code cell
    let code_cell = notebook
        .cells
        .iter()
        .find(|cell| matches!(cell, Cell::Code { .. }))
        .unwrap();
    if let Cell::Code {
        id,
        metadata,
        execution_count,
        source,
        outputs,
    } = code_cell
    {
        assert!(id.is_some());

        assert!(*id == Some(CellId("38f37a24".to_string())));
        // assert!(metadata.id.is_some());
        assert!(execution_count.is_some());
        assert!(!source.is_empty());
        assert!(!outputs.is_empty());
    } else {
        panic!("Expected code cell");
    }

    // Check a markdown cell
    let markdown_cell = notebook
        .cells
        .iter()
        .find(|cell| matches!(cell, Cell::Markdown { .. }))
        .unwrap();
    if let Cell::Markdown {
        id,
        metadata,
        source,
        attachments,
    } = markdown_cell
    {
        assert!(id.is_some());
        // assert!(metadata.id.is_some());
        assert!(!source.is_empty());
        assert!(attachments.is_none() || attachments.as_ref().unwrap().is_object());
    } else {
        panic!("Expected markdown cell");
    }
}

#[test]
fn test_open_all_notebooks_in_dir() {
    let dir = Path::new("tests/notebooks");
    for entry in fs::read_dir(dir).expect("Failed to read directory") {
        let entry = entry.expect("Failed to read entry");
        let path = entry.path();
        let path_str = path.to_str().expect("Failed to convert path to string");
        if path_str.ends_with(".ipynb") {
            let notebook_json = read_notebook(path_str);
            let notebook: Notebook = serde_json::from_str(&notebook_json)
                .expect(&format!("Failed to parse notebook: {}", path_str));
            assert_eq!(notebook.nbformat, 4);
        }
    }
}
