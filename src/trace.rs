use std::future::Future;
use tracing::Instrument;

#[derive(Clone, Debug, Default)]
pub struct TraceExecutor(());

impl TraceExecutor {
    pub fn new() -> Self {
        Self(())
    }
}

impl<F> hyper::rt::Executor<F> for TraceExecutor
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    #[inline]
    fn execute(&self, f: F) {
        tokio::spawn(f.in_current_span());
    }
}