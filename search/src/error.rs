#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("Invalid date {year:02}-{month:02}-{day:02}")]
    InvalidDate { year: u8, month: u8, day: u8 },
    #[error("Summer month {0} is excluded from this search")]
    SummerMonth(u8),
    #[error("Unsupported Tepig nature {0} (expected Naughty or Rash)")]
    UnsupportedNature(u8),
    #[error(transparent)]
    GpuInitialization(#[from] infra::gpu::context::GpuInitError),
    #[error(transparent)]
    GpuExecution(#[from] infra::gpu::readback::ReadbackError),
    #[error("Failed to start CPU worker: {0}")]
    WorkerStart(#[source] std::io::Error),
    #[error("CPU worker thread panicked")]
    WorkerPanicked,
    #[error("Search cancelled")]
    Cancelled,
}
