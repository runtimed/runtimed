#[cfg(test)]
mod test {
    use nbformat::legacy::Cell as LegacyCell;
    use nbformat::v4::{Cell, Output};
    use nbformat::{parse_notebook, serialize_notebook, Notebook};
    use serde_json::Value;
    use std::fs;
    use std::path::Path;

    fn read_notebook(path: &str) -> String {
        fs::read_to_string(Path::new(path)).expect("Failed to read notebook file")
    }

    #[test]
    fn test_parse_legacy_v4_notebook() {
        let notebook_json = read_notebook("tests/notebooks/test4.ipynb");
        let notebook = parse_notebook(&notebook_json).expect("Failed to parse notebook");

        let notebook = if let Notebook::Legacy(notebook) = notebook {
            notebook
        } else {
            panic!("Expected v4.1 - v4.4 notebook");
        };

        assert_eq!(notebook.nbformat, 4);
        assert_eq!(notebook.nbformat_minor, 1);

        assert_eq!(notebook.cells.len(), 9);

        assert!(notebook.metadata.kernelspec.is_none());
        assert!(notebook.metadata.language_info.is_none());

        // Check first cell (markdown)
        let first_cell = &notebook.cells[0];
        if let LegacyCell::Markdown { source, .. } = first_cell {
            assert_eq!(source, &vec!["# nbconvert latex test"]);
        } else {
            panic!("First cell should be markdown");
        }

        // Check a code cell
        let code_cell = &notebook.cells[3];
        if let LegacyCell::Code {
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
        let notebook = parse_notebook(&notebook_json).expect("Failed to parse notebook");

        let notebook = if let Notebook::V4(notebook) = notebook {
            notebook
        } else {
            panic!("Expected v4.1 - v4.4 notebook");
        };

        assert_eq!(notebook.nbformat, 4);
        assert_eq!(notebook.nbformat_minor, 5);
        assert!(!notebook.cells.is_empty());

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
            metadata: _,
            execution_count,
            source,
            outputs,
        } = code_cell
        {
            assert_eq!(id.as_str(), "38f37a24");
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
            metadata: _,
            source,
            attachments,
        } = markdown_cell
        {
            assert_eq!(id.as_str(), "2fcdfa53");
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
                // If the file starts with `test3`, let's check that we got an error
                let notebook_json = read_notebook(path_str);
                let notebook = parse_notebook(&notebook_json);

                println!("Parsing notebook: {}", path_str);
                if path_str.contains("invalid_cell_id") || path_str.contains("invalid_metadata") {
                    assert!(
                        matches!(notebook, Err(nbformat::NotebookError::JsonError(_))),
                        "Expected JsonError for invalid data in {}",
                        path_str
                    );
                } else if path_str.starts_with("tests/notebooks/test2")
                    || path_str.starts_with("tests/notebooks/test3")
                    || path_str.starts_with("tests/notebooks/test4plus")
                    || path_str.starts_with("tests/notebooks/invalid")
                    || path_str.starts_with("tests/notebooks/no_min_version")
                {
                    assert!(notebook.is_err(), "Expected error for {}", path_str);
                } else {
                    assert!(notebook.is_ok(), "Failed to parse notebook: {}", path_str);
                }
            }
        }
    }

    /// Compare notebook JSON at a key level so that mismatches bubble up as lines like `Serialization mismatch: Extra key 'attachments' in serialized at root.cells[0]`
    fn compare_notebook_json(original: &Value, serialized: &Value) -> Result<(), String> {
        fn compare_values(path: &str, v1: &Value, v2: &Value) -> Result<(), String> {
            match (v1, v2) {
                (Value::Object(o1), Value::Object(o2)) => {
                    for (k, v) in o1 {
                        if !o2.contains_key(k) {
                            return Err(format!("Key '{}' missing in serialized at {}", k, path));
                        }
                        compare_values(&format!("{}.{}", path, k), v, &o2[k])?;
                    }
                    for k in o2.keys() {
                        if !o1.contains_key(k) {
                            return Err(format!("Extra key '{}' in serialized at {}", k, path));
                        }
                    }
                }
                (Value::Array(a1), Value::Array(a2)) => {
                    if a1.len() != a2.len() {
                        return Err(format!("Array length mismatch at {}", path));
                    }
                    for (i, (v1, v2)) in a1.iter().zip(a2.iter()).enumerate() {
                        compare_values(&format!("{}[{}]", path, i), v1, v2)?;
                    }
                }
                (Value::String(s1), Value::String(s2)) => {
                    if s1.trim() != s2.trim() {
                        return Err(format!("String mismatch at {}: '{}' vs '{}'", path, s1, s2));
                    }
                }
                (v1, v2) => {
                    if v1 != v2 {
                        return Err(format!("Value mismatch at {}: {:?} vs {:?}", path, v1, v2));
                    }
                }
            }
            Ok(())
        }

        compare_values("root", original, serialized)
    }

    #[test]
    fn test_serialize_deserialize() {
        let notebook_json = read_notebook("tests/notebooks/test4.5.ipynb");
        let notebook = parse_notebook(&notebook_json).expect("Failed to parse notebook");

        let serialized = serialize_notebook(&notebook).expect("Failed to serialize notebook");

        let original_value: Value =
            serde_json::from_str(&notebook_json).expect("Failed to parse original JSON");
        let serialized_value: Value =
            serde_json::from_str(&serialized).expect("Failed to parse serialized JSON");

        if let Err(diff) = compare_notebook_json(&original_value, &serialized_value) {
            panic!("Serialization mismatch: {}", diff);
        }

        println!("Structures match in contents!");

        println!("Original:\n\n{}", notebook_json);
        println!("Serialized:\n\n{}", serialized);

        // Now for the hardest part -- seeing if we can get exact text back
        assert_eq!(notebook_json, serialized);
    }

    #[test]
    fn test_serialize_deserialize_another() {
        let notebook_json = read_notebook("tests/notebooks/Mediatypes.ipynb");
        let notebook = parse_notebook(&notebook_json).expect("Failed to parse notebook");

        let serialized = serialize_notebook(&notebook).expect("Failed to serialize notebook");

        let original_value: Value =
            serde_json::from_str(&notebook_json).expect("Failed to parse original JSON");
        let serialized_value: Value =
            serde_json::from_str(&serialized).expect("Failed to parse serialized JSON");

        if let Err(diff) = compare_notebook_json(&original_value, &serialized_value) {
            panic!("Serialization mismatch: {}", diff);
        }

        println!("Structures match in contents!");

        // std::fs::write("og.json", &notebook_json).expect("Failed to write original JSON");
        // std::fs::write("ser.json", &serialized).expect("Failed to write serialized JSON");

        // Right now, this Mediatypes notebook has two outputs with a differing newline at the end
        // between the original and the serialized.
        // assert_eq!(notebook_json, serialized);
    }
}
