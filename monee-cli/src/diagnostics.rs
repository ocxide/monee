pub fn snapshot_read_diagnostic(error: monee::snapshot_io::ReadError) -> miette::Report {
    use monee::snapshot_io;

    match error {
        snapshot_io::ReadError::Io(err) => miette::diagnostic!(
            severity = miette::Severity::Error,
            code = "io::ReadError",
            "{err}",
        )
        .into(),
        snapshot_io::ReadError::Json(err) => {
            let diagnostic: crate::json_diagnostic::JsonDecodeDiagnostic = err.into();
            diagnostic.into()
        }
    }
}

pub fn apply_diagnostic(why: monee_core::Error) -> miette::MietteDiagnostic {
    let diagnostic = miette::diagnostic!(
        severity = miette::Severity::Error,
        code = "event::ApplyError",
        "{why:?}",
    );
    diagnostic
}

pub fn snapshot_r_diagnostic(err: monee::error::SnapshotReadError) -> miette::Report {
    use monee::error::SnapshotReadError as Error;

    match err {
        Error::Read(e) => crate::diagnostics::snapshot_read_diagnostic(e),
        Error::SnapshotApply(e) => apply_diagnostic(e).into(),
        Error::Database(e) => monee::log::database(e),
    }
}

pub fn snapshot_opt_diagnostic(err: monee::error::SnapshotOptError) -> miette::Report {
    use monee::error::SnapshotOptError as Error;

    match err {
        Error::Read(e) => crate::diagnostics::snapshot_read_diagnostic(e),
        Error::Write(e) => write_diagnostic(e),
        Error::Database(e) => monee::log::database(e),
        Error::SnapshotApply(e) => apply_diagnostic(e).into(),
    }
}

fn write_diagnostic(error: std::io::Error) -> miette::Report {
    use std::io::ErrorKind;

    match error.kind() {
        ErrorKind::PermissionDenied => miette::diagnostic!(
            severity = miette::Severity::Error,
            code = "io::WriteError",
            "Permission denied",
        )
        .into(),
        _ => monee::log::snapshot_write(error),
    }
}
