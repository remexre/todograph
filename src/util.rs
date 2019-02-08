use futures::{future::poll_fn, Future};
use log::error;
use std::error::Error;

/// A convenient alias for `Result`.
pub type Result<T, E = Box<dyn Error + Send + Sync + 'static>> = ::std::result::Result<T, E>;

/// A higher-level version of `tokio_threadpool::blocking`.
pub fn blocking<E, F, T>(func: F) -> impl Future<Item = T, Error = E>
where
    F: FnOnce() -> Result<T, E>,
{
    let mut func = Some(func);
    poll_fn(move || {
        tokio_threadpool::blocking(|| (func.take().unwrap())())
            .map_err(|_| panic!("Blocking operations must be run inside a Tokio thread pool!"))
    })
    .and_then(|r| r)
}

/// Logs an error, including its causes.
pub fn log_err(err: &(dyn Error + 'static)) {
    // Count the number of errors.
    let num_errs = ErrorCauseIter::from(err).count();

    if num_errs <= 1 {
        error!("{}", err);
    } else {
        let mut first = true;
        for err in ErrorCauseIter::from(err) {
            if first {
                first = false;
                error!("           {}", err);
            } else {
                error!("caused by: {}", err);
            }
        }
    }
}

/// An iterator over the causes of an error.
#[derive(Debug)]
pub struct ErrorCauseIter<'a>(Option<&'a (dyn Error + 'static)>);

impl<'a> From<&'a (dyn Error + 'static)> for ErrorCauseIter<'a> {
    fn from(err: &'a (dyn Error + 'static)) -> ErrorCauseIter<'a> {
        ErrorCauseIter(Some(err))
    }
}

impl<'a> Iterator for ErrorCauseIter<'a> {
    type Item = &'a dyn Error;

    fn next(&mut self) -> Option<&'a dyn Error> {
        let err = self.0?;
        self.0 = err.source();
        Some(err)
    }
}

/// An explicit trivial cast.
#[macro_export]
macro_rules! coerce {
    ($e:expr => $t:ty) => {{
        let x: $t = $e;
        x
    }};
}
