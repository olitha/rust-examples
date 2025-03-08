use std::time::Duration;

use tracing::Instrument;

// async fn main() {
//     // tracing().await;
//     log().await;
// }

async fn task() {
    for _ in 0..2 {
        log::info!("task");
        tokio::time::sleep(Duration::from_millis(5)).await;
        async {
            log::info!("subtask");
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        .await;
    }
}

// #[tokio::main]
async fn _main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .format_timestamp(None)
        .init();

    tokio::join!(task(), task());
}

async fn tracing_task() {
    for _ in 0..2 {
        tracing::info!("task");
        tokio::time::sleep(Duration::from_millis(5)).await;
        async {
            tracing::info!("subtask");
        }
        .instrument(tracing::info_span!("task inner"))
        .await;
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::fmt().without_time().init();

    tokio::join!(
        tracing_task().instrument(tracing::info_span!("task", position = 1)),
        tracing_task().instrument(tracing::info_span!("task", position = 2))
    );
}
