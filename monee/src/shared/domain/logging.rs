use crate::shared::infrastructure::errors::InfrastructureError;

pub trait LogRepository {
    fn log(&self, message: std::fmt::Arguments) -> Result<(), InfrastructureError>;
}

