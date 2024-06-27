pub fn snapshot_read_diagnostic(
    error: twon::snapshot_io::ReadError,
) -> miette::Report {
    use twon::snapshot_io;

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

pub fn apply_diagnostic(why: twon_core::Error) -> miette::MietteDiagnostic {
    let diagnostic = miette::diagnostic!(
        severity = miette::Severity::Error,
        code = "event::ApplyError",
        "{why:?}",
    );
    diagnostic
}

pub fn snapshot_r_diagnostic(err: twon::error::SnapshotReadError) -> miette::Report {
    use twon::error::SnapshotReadError as Error;

    match err {
        Error::Read(e) => crate::diagnostics::snapshot_read_diagnostic(e),
        Error::SnapshotApply(e) => apply_diagnostic(e).into(),
        Error::Database(e) => twon::log::database(e),
    }
}

pub fn snapshot_opt_diagnostic(err: twon::error::SnapshotOptError) -> miette::Report {
    use twon::error::SnapshotOptError as Error;

    match err {
        Error::Read(e) => crate::diagnostics::snapshot_read_diagnostic(e),
        Error::Write(e) => twon::log::snapshot_write(e),
        Error::Database(e) => twon::log::database(e),
        Error::SnapshotApply(e) => apply_diagnostic(e).into(),
    }
}

pub fn snapshot_write_diagnostic(
    err: twon::error::SnapshotWriteError,
) -> miette::Report {
    use twon::error::SnapshotWriteError as Error;

    match err {
        Error::Write(e) => twon::log::snapshot_write(e),
        Error::Database(e) => twon::log::database(e),
        Error::SnapshotApply(e) => apply_diagnostic(e).into(),
    }
}
