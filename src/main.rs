mod server;
mod pdf;
mod ai;
mod ocr;
mod final_challenge;

use axum::{
    routing::post,
    Router,
};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing_subscriber;
use tokio::sync::oneshot;
use std::io::{self, Write};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    loop {
        println!("==== HackRX API Menu ====");
        println!("1. Start Server");
        println!("2. Show Server Status (placeholder)");
        println!("3. Exit");
        print!("Enter your choice: ");
        io::stdout().flush()?; // flush to ensure prompt appears

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let choice = input.trim();

        match choice {
            "1" => {
                println!("Starting server on http://0.0.0.0:8000 ... Press Ctrl+C to stop.");

                // Spawn server task with a shutdown signal for clean exit
                let app = Router::new().route("/api/v1/hackrx/run", post(server::hackrx_run));
                let addr: SocketAddr = "0.0.0.0:8000".parse()?;
                let listener = TcpListener::bind(addr).await?;

                // Use a one-shot channel for shutdown signal (not hooked here, but ready to use)
                let (_shutdown_tx, _shutdown_rx) = oneshot::channel::<()>();

                // Run server in current task - this will block until Ctrl+C or shutdown signal
                println!("Server running... Press Ctrl+C to stop.");

                let server = axum::serve(listener, app);

                // Await the server future or a Ctrl+C signal for graceful shutdown
                tokio::select! {
                    res = server => {
                        if let Err(err) = res {
                            eprintln!("Server error: {}", err);
                        }
                    }
                    _ = tokio::signal::ctrl_c() => {
                        println!("Ctrl+C received, shutting down...");
                    }
                }

                // At this point, the server has stopped, so back to menu
            }
            "2" => {
                println!("Server status feature not implemented yet.");
                // You can add any other functionality here
            }
            "3" => {
                println!("Exiting program. Goodbye!");
                break; // exit loop and program
            }
            _ => {
                println!("Invalid option, please enter 1, 2 or 3.");
            }
        }

        println!(); // blank line for readability
    }

    Ok(())
}
