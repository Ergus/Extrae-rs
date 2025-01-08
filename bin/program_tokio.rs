use extrae_rs::ExtraeSubscriber;

//use tracing::{info, Level};
use tracing::subscriber::set_global_default;
use tokio::task;
use tokio::time::{self, Duration};

#[tracing::instrument]
async fn task1() {
    //info!("Task 1 started");
    time::sleep(Duration::from_millis(500)).await;
    //info!("Task 1 completed");
}

#[tracing::instrument(name = "custom_task2", skip(_param))]
async fn task2(_param: &str) {
    //info!(param, "Task 2 started");
    time::sleep(Duration::from_millis(300)).await;
    //info!("Task 2 completed");
}


#[tokio::main]
async fn main() {

    // Set up a subscriber that logs to stdout
    let subscriber = ExtraeSubscriber::new();
    set_global_default(subscriber).expect("Could not set global default subscriber");

    // Run tasks concurrently
    let handle1 = task::spawn(task1());
    let handle2 = task::spawn(task2("Hello"));

    // let handle3 = task::spawn(task1());
    // let handle4 = task::spawn(task2("World"));

    // Wait for both tasks to complete
    let _ = tokio::join!(handle1, handle2);
}
