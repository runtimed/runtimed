use core::fmt;

#[derive(Debug)]
pub enum JupyterError {
    ParseError {
        msg_type: Option<String>,
        source: serde_json::Error,
    },
}

impl core::error::Error for JupyterError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            JupyterError::ParseError { source, .. } => Some(source),
        }
    }
}

impl fmt::Display for JupyterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JupyterError::ParseError {
                msg_type: Some(msg_type),
                source,
            } => {
                write!(
                    f,
                    "Error deserializing content for msg_type `{}`: {}",
                    msg_type, source
                )
            }
            JupyterError::ParseError {
                msg_type: None,
                source,
            } => {
                write!(f, "{}", source)
            }
        }
    }
}

impl From<serde_json::Error> for JupyterError {
    fn from(source: serde_json::Error) -> Self {
        JupyterError::ParseError {
            msg_type: None,
            source,
        }
    }
}
