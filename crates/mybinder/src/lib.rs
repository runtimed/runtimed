use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
pub struct BinderResponse {
    pub phase: String,
    pub message: Option<String>,
    pub image: Option<String>,
    pub url: Option<String>,
}

pub fn parse_binder_response(line: &str) -> Result<BinderResponse> {
    if let Some(json_str) = line.strip_prefix("data: ") {
        let response: BinderResponse = serde_json::from_str(json_str)?;
        Ok(response)
    } else {
        Err(anyhow::anyhow!("Invalid response format"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_parse_successful_launch() {
        let content = fs::read_to_string("fixtures/success-launch").unwrap();
        let lines: Vec<&str> = content.lines().collect();

        let first_response = parse_binder_response(lines[0]).unwrap();
        assert_eq!(first_response.phase, "built");
        assert_eq!(
            first_response.message,
            Some("Found built image, launching...\n".to_string())
        );

        let last_response = parse_binder_response(lines.last().unwrap()).unwrap();
        assert_eq!(last_response.phase, "ready");
        assert!(last_response.url.is_some());
        assert!(last_response.image.is_some());
    }

    #[test]
    fn test_parse_failed_launch() {
        let content = fs::read_to_string("fixtures/failed-launch").unwrap();
        let lines: Vec<&str> = content.lines().collect();

        let first_response = parse_binder_response(lines[0]).unwrap();
        assert_eq!(first_response.phase, "waiting");

        let failed_response = parse_binder_response(
            lines
                .iter()
                .rev()
                .find(|&line| line.contains("\"phase\": \"failed\""))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(failed_response.phase, "failed");
    }
}

