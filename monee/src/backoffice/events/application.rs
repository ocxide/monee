pub mod add {
    use cream::{context::FromContext, events::bus::EventBusPort};
    use monee_core::EventId;
    use monee_types::backoffice::events::event_added::EventAdded;

    use crate::{
        backoffice::{
            events::domain::{apply_event, repository::Repository},
            snapshot::application::snapshot_io::SnapshotIO,
        },
        shared::{domain::context::AppContext, infrastructure::errors::AppError},
    };

    pub use crate::backoffice::events::domain::event::Event;
    pub use apply_event::{Error, MoveValueError};

    #[derive(FromContext)]
    #[context(AppContext)]
    pub struct Add {
        repository: Box<dyn Repository>,
        snapshot_io: SnapshotIO,
        port: EventBusPort,
    }

    impl Add {
        pub async fn run(&self, event: Event) -> Result<(), AppError<Error>> {
            let mut snapshot = self.snapshot_io.read_last().await?;
            if let Err(e) = apply_event::apply_event(&mut snapshot, &event) {
                return Err(AppError::App(e));
            }

            let id = EventId::default();

            self.repository.add(id, event).await?;
            self.snapshot_io.save(snapshot).await?;

            self.port.publish(EventAdded { id });

            Ok(())
        }
    }
}
