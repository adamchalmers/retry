/// If the future eventually resolves a value that passes the test, it returns it, along with some
/// metrics. This struct combines the value returned, along with how long/how many restarts it took
/// to get that value.
#[derive(Debug)]
pub struct Success<T> {
    /// The success value returned by the test
    pub value: T,
    /// How much time elapsed while waiting for the future to successfully resolve
    pub duration: std::time::Duration,
    /// How many times the future needed to be restarted before it successfully resolved
    pub restarts: usize,
}

/// Different ways a Restartable can fail
#[derive(Debug)]
pub enum Failure<E> {
    /// Returned if the inner future never resolved before the timeout
    Timeout,
    /// Returned if the inner future fails the test and then times out. Returns the last error
    /// from the test, and how many times the future was restarted.
    Err {
        /// The failure value returne by the test
        error: E,
        /// How many times the future was restarted before the timeout expired
        restarts: usize,
    },
}
