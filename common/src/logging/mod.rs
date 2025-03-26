use crate::config::LogConfig;
use time::UtcOffset;
use time::format_description::BorrowedFormatItem;
use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
use tracing_appender::rolling;
use tracing_appender::rolling::Rotation;
use tracing_subscriber::fmt::time::OffsetTime;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, fmt};

pub fn init_log(config: LogConfig, file_name: String) -> anyhow::Result<Vec<WorkerGuard>> {
    let timer = cre_timer()?;
    let file_appender = rolling::Builder::new()
        .rotation(Rotation::DAILY)
        .max_log_files(config.max_file as usize)
        .filename_prefix(file_name)
        .filename_suffix("log")
        .build(config.dir)?;
    let mut guards = vec![];
    let (non_blocking_file, guard) = NonBlocking::new(file_appender);
    let file_layer = fmt::layer()
        .with_writer(non_blocking_file)
        .with_timer(timer)
        .with_line_number(true)
        .with_ansi(false);
    guards.push(guard);
    
    let console_layer = match config.console {
        true => {
            let timer = cre_timer()?;
            let (non_blocking, guard) = tracing_appender::non_blocking(std::io::stdout());
            guards.push(guard);
            Some(
                fmt::layer()
                    .with_writer(non_blocking)
                    .with_timer(timer)
                    .with_line_number(true)
                    .with_ansi(true),
            )
        }
        false => None,
    };

    match console_layer {
        Some(layer) => {
            tracing_subscriber::registry()
                .with(file_layer)
                .with(layer)
                .with(EnvFilter::builder().parse(config.level.to_uppercase())?)
                .try_init()?;
        }
        None => {
            tracing_subscriber::registry()
                .with(file_layer)
                .with(EnvFilter::builder().parse(config.level.to_uppercase())?)
                .try_init()?;
        }
    }
    Ok(guards)
}
#[inline]
fn cre_timer() -> anyhow::Result<OffsetTime<Vec<BorrowedFormatItem<'static>>>> {
    let offset = UtcOffset::current_local_offset()?;
    let time_format = time::format_description::parse(
        "[year]-[month padding:zero]-[day padding:zero] [hour]:[minute]:[second].[subsecond digits:3]",
    )?;
    Ok(OffsetTime::new(offset, time_format))
}

mod test {
    use crate::config::LogConfig;
    use crate::logging::init_log;

    #[test]
    fn it_works() {
        let lod_guards = init_log(
            LogConfig {
                console: true,
                level: "debug".to_string(),
                dir: "./logs".to_string(),
                max_file: 1,
            },
            "test".to_string(),
        )
        .expect("logging initialization failed");
        tracing::info!("This is a tracing info message");
        log::info!("This is a log info message");
    }
}
