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

pub fn snapshot_opt_diagnostic(err: twon_persistence::error::SnapshotOptError) -> miette::Report {
    use twon_persistence::error::SnapshotOptError as Error;

    match err {
        Error::Read(e) => crate::diagnostics::snapshot_read_diagnostic(e),
        Error::Write(e) => twon_persistence::log::snapshot_write(e),
        Error::Database(e) => twon_persistence::log::database(e),
        Error::SnapshotApply(e) => apply_diagnostic(e).into(),
    }
}

pub fn snapshot_write_diagnostic(
    err: twon_persistence::error::SnapshotWriteError,
) -> miette::Report {
    use twon_persistence::error::SnapshotWriteError as Error;

    match err {
        Error::Write(e) => twon_persistence::log::snapshot_write(e),
        Error::Database(e) => twon_persistence::log::database(e),
        Error::SnapshotApply(e) => apply_diagnostic(e).into(),
    }
}
