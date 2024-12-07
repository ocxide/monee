pub mod application {
    pub mod logging {
        use cream::context::FromContext;

        use crate::shared::{
            domain::{context::AppContext, logging::LogRepository},
            infrastructure::errors::InfrastructureError,
        };

        #[derive(FromContext)]
        #[context(AppContext)]
        pub struct LogService {
            repository: Box<dyn LogRepository>,
        }

        impl LogService {
            pub fn error(&self, err: InfrastructureError) {
                let result = self.repository.log(format_args!("{:?}", err));
                if let Err(e) = result {
                    println!("error logging error: {:?}", e);
                }
            }
        }
    }
}

pub mod domain;
pub mod infrastructure;
