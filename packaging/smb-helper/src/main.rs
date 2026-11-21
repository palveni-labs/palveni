mod access;
mod selection;
mod wire;

fn main() -> std::process::ExitCode {
    let result = tokio::runtime::Builder::new_current_thread()
        .max_blocking_threads(2)
        .enable_all()
        .build()
        .and_then(|runtime| runtime.block_on(access::run()));
    if result.is_ok() {
        std::process::ExitCode::SUCCESS
    } else {
        std::process::ExitCode::FAILURE
    }
}
