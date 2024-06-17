use std::io::Write;

const FILE: &str = "twon.log";

fn write_error_log<E: std::error::Error>(error: E) {
    let path = crate::create_local_path().join(FILE);
    let mut file = match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        Ok(file) => file,
        Err(e) => {
            println!("WARNING - Unable to open log file: {}", e);
            return;
        }
    };

    let now = crate::Timezone::now();
    let result = writeln!(
        file,
        "ERROR {} - {} {}:{} - {error:?}",
        now.format("%d/%m/%Y %H:%M"),
        file!(),
        line!(),
        column!()
    );

    if let Err(e) = result {
        println!("WARNING - Unable to write to log file: {}", e);
    }
}

pub fn database(error: surrealdb::Error) -> ! {
    write_error_log(error);
    panic!("Error: Database error, aborting...");
}

pub fn snapshot_read(error: std::io::Error) -> ! {
    write_error_log(error);
    panic!("Error: Snapshot read error, aborting...");
}

pub fn snapshot_write(error: std::io::Error) -> ! {
    write_error_log(error);
    panic!("Error: Snapshot write error, aborting...");
}

