use crate::backoff::Fixed;
use crate::iter::RetryIter;
use crate::jitter::NoJitter;

/// Create a sync retry iterator with default settings (no delay, no limit).
pub fn iter() -> RetryIter<Fixed, NoJitter> {
    RetryIter::new()
}
