use crate::models::date_time_error::DateTimeError;
use infra::gpu::readback::ReadbackError;

#[derive(Debug, thiserror::Error)]
pub enum GpuResultError {
    #[error(transparent)]
    Readback(#[from] ReadbackError),
    #[error("GPU candidate contains an invalid date or time: {0}")]
    InvalidCandidate(#[from] DateTimeError),
}
