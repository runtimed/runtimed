use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "phase", rename_all = "lowercase")]
pub enum Phase {
    Built {
        message: Option<String>,
        #[serde(rename = "imageName")]
        image_name: Option<String>,
    },
    Launching {
        message: Option<String>,
    },
    Ready {
        message: Option<String>,
        url: String,
        token: String,
        image: String,
        #[serde(rename = "repo_url")]
        repo_url: String,
        #[serde(rename = "binder_ref_url")]
        binder_ref_url: String,
        #[serde(rename = "binder_launch_host")]
        binder_launch_host: String,
        #[serde(rename = "binder_request")]
        binder_request: String,
        #[serde(rename = "binder_persistent_request")]
        binder_persistent_request: String,
    },
    Failed {
        message: Option<String>,
    },
    Waiting {
        message: Option<String>,
    },
    Fetching {
        message: Option<String>,
    },
    Building {
        message: Option<String>,
    },
    Unknown {
        message: Option<String>,
    },
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct BinderBuildResponse {
    #[serde(flatten)]
    pub phase: Phase,
}

pub fn parse_binder_build_response(line: &str) -> Result<BinderBuildResponse> {
    if let Some(json_str) = line.strip_prefix("data: ") {
        let response: BinderBuildResponse = serde_json::from_str(json_str)?;
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

        let first_response = parse_binder_build_response(lines[0]).unwrap();
        if let Phase::Built {
            message,
            image_name,
        } = first_response.phase
        {
            assert_eq!(
                message,
                Some("Found built image, launching...\n".to_string())
            );
            assert!(image_name.is_some() && !image_name.unwrap().is_empty());
        } else {
            panic!("Expected Built phase");
        }

        let last_response = parse_binder_build_response(lines.last().unwrap()).unwrap();
        if let Phase::Ready { url, token, .. } = last_response.phase {
            assert_eq!(token, "T3VMkxCnQamsWGL9-j1fMQ");
            assert_eq!(url, "https://notebooks.gesis.org/binder/jupyter/user/binder-examples-nda_environment-iy8slc0g/");
        } else {
            panic!("Expected Ready phase");
        }
    }

    #[test]
    fn test_parse_failed_launch() {
        let content = fs::read_to_string("fixtures/failed-launch").unwrap();
        let lines: Vec<&str> = content.lines().collect();

        let first_response = parse_binder_build_response(lines[0]).unwrap();
        assert!(matches!(first_response.phase, Phase::Waiting { .. }));

        let failed_response = parse_binder_build_response(
            lines
                .iter()
                .rev()
                .find(|&line| line.contains("\"phase\": \"failed\""))
                .unwrap(),
        )
        .unwrap();
        assert!(matches!(failed_response.phase, Phase::Failed { .. }));
    }
}
