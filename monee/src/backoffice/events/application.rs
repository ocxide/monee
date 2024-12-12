pub mod add {
    use cream::context::FromContext;

    use crate::{
        backoffice::{
            events::domain::{
                apply_event::{apply_event, Error},
                event::Event,
                repository::Repository,
            },
            snapshot::application::snapshot_io::SnapshotIO,
        },
        shared::{domain::context::AppContext, infrastructure::errors::AppError},
    };

    #[derive(FromContext)]
    #[context(AppContext)]
    pub struct Add {
        repository: Box<dyn Repository>,
        snapshot_io: SnapshotIO,
    }

    impl Add {
        pub async fn run(&self, event: Event) -> Result<(), AppError<Error>> {
            let mut snapshot = self.snapshot_io.read_last().await?;
            if let Err(e) = apply_event(&mut snapshot, &event) {
                return Err(AppError::App(e));
            }

            self.repository.add(event).await?;
            self.snapshot_io.save(snapshot).await?;

            Ok(())
        }
    }
}

