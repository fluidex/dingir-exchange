use tracing_appender::non_blocking::WorkerGuard;

// NOTE: The return WorkerGuard MUST be hold by caller, otherwise the tracing thread is dropped.
pub fn setup() -> WorkerGuard {
    let worker_guard = init_non_blocking_tracing();
    set_panic_hook();
    worker_guard
}

fn init_non_blocking_tracing() -> WorkerGuard {
    let (non_blocking, worker_guard) = tracing_appender::non_blocking(std::io::stdout());
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(non_blocking)
        .init();
    worker_guard
}

pub fn set_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let location = info.location().unwrap();

        let message = match info.payload().downcast_ref::<&'static str>() {
            Some(s) => *s,
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => &s[..],
                None => "Box<dyn Any>",
            },
        };

        let thread = std::thread::current();
        let thread_name = thread.name().unwrap_or("<unnamed>");

        let overall = format!(
            "thread '{}' panicked at '{}', {}\n{}",
            thread_name,
            message,
            location,
            get_backtrace(),
        );

        log::error!("{}", overall);
    }));
}

fn get_backtrace() -> String {
    match std::env::var("RUST_BACKTRACE").ok() {
        None => "".to_owned(),
        Some(ref val) if val == "0" => "".to_owned(),
        _ => format!("{:?}", backtrace::Backtrace::new()),
    }
}
