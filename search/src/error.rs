#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error(transparent)]
    InvalidDateTime(#[from] rng_core::models::date_time_error::DateTimeError),
    #[error("Summer month {0} is excluded from this search")]
    SummerMonth(u8),
    #[error("Unsupported Tepig nature {0} (expected Naughty or Rash)")]
    UnsupportedNature(u8),
    #[error(transparent)]
    GpuInitialization(#[from] rng_gpu::context::GpuInitError),
    #[error(transparent)]
    GpuExecution(#[from] rng_gpu::error::GpuResultError),
    #[error("Failed to start CPU worker: {0}")]
    WorkerStart(#[source] std::io::Error),
    #[error("CPU worker thread panicked")]
    WorkerPanicked,
    #[error("Search cancelled")]
    Cancelled,
}
