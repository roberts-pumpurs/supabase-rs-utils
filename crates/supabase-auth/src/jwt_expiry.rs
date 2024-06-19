use std::task::{Context, Poll};
use std::time::Duration;

use futures::Future;
use futures_timer::Delay;
use pin_project::pin_project;

#[pin_project]
pub(crate) struct JwtExpiry {
    #[pin]
    delay: Delay,
}

impl JwtExpiry {
    pub(crate) fn new(valid_for: Duration) -> Self {
        Self {
            delay: Delay::new(valid_for),
        }
    }
}

impl Future for JwtExpiry {
    type Output = ();

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.project().delay.poll(cx)
    }
}

#[cfg(test)]
mod tests {
    use std::task::Context;
    use std::time::Instant;

    use futures::executor::block_on;
    use futures::future::poll_fn;
    use futures::task::noop_waker;
    use futures::FutureExt;

    use super::*;

    #[rstest::rstest]
    #[tokio::test]
    #[timeout(Duration::from_secs(2))]
    async fn test_jwt_expiry_completes() {
        let duration = Duration::from_millis(100);
        let now = Instant::now();
        let jwt_expiry = JwtExpiry::new(duration);
        jwt_expiry.await;
        let elapsed = now.elapsed();

        assert!(elapsed >= duration);
    }

    #[rstest::rstest]
    #[timeout(Duration::from_secs(2))]
    fn test_jwt_expiry_does_not_complete_before_duration() {
        let duration = Duration::from_millis(100);
        let mut jwt_expiry = JwtExpiry::new(duration);

        let waker = noop_waker();
        let cx = Context::from_waker(&waker);

        let start = Instant::now();
        let mut polled_once = false;

        let poll_result = poll_fn(|cx| {
            let poll_result = jwt_expiry.poll_unpin(cx);
            if !polled_once {
                assert!(start.elapsed() < duration);
                polled_once = true;
            }
            poll_result
        });

        block_on(poll_result);

        let elapsed = start.elapsed();
        assert!(elapsed >= duration);
    }
}
