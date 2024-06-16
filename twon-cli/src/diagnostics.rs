pub fn snapshot_read_diagnostic(
    error: twon_persistence::snapshot_io::read::Error,
) -> miette::Report {
    use twon_persistence::snapshot_io;

    match error {
        snapshot_io::read::Error::Io(err) => miette::diagnostic!(
            severity = miette::Severity::Error,
            code = "io::ReadError",
            "{err}",
        )
        .into(),
        snapshot_io::read::Error::Json(err) => {
            let diagnostic: crate::json_diagnostic::JsonDecodeDiagnostic = err.into();
            diagnostic.into()
        }
    }
}

pub fn apply_diagnostic(why: twon_core::Error) -> miette::MietteDiagnostic {
    let diagnostic = miette::diagnostic!(
        severity = miette::Severity::Error,
        code = "event::ApplyError",
        "{why:?}",
    );
    diagnostic
}

