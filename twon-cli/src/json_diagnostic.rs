use twon::snapshot_io;

#[derive(miette::Diagnostic, Debug)]
#[diagnostic(code = "snapshot::DecodeError", severity(Error))]
pub struct JsonDecodeDiagnostic {
    error: serde_json::Error,
    #[source_code]
    source: miette::NamedSource<String>,
    #[label = "{error}"]
    label: (usize, usize),
}

impl std::fmt::Display for JsonDecodeDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Failed to decode snapshot")
    }
}

impl std::error::Error for JsonDecodeDiagnostic {}

impl From<snapshot_io::JsonDecodeError> for JsonDecodeDiagnostic {
    fn from(err: snapshot_io::JsonDecodeError) -> Self {
        let snapshot_io::JsonDecodeError {
            error,
            json,
            filename,
        } = err;

        let start = json
            .lines()
            .take(error.line() - 1)
            .map(|line| line.chars().count())
            .sum::<usize>()
            + error.column()
            - 1;

        let label = (start, start);

        Self {
            error,
            source: miette::NamedSource::new(filename.to_string_lossy(), json),
            label,
        }
    }
}

