//! PIR Sensor Test Binary
//!
//! Tests PIR motion sensor with timeout detection.
//! Demonstrates both PresenceDetected and NoPresenceSince events.

use anyhow::{Context, Result};
use sensors::PirSensorController;
use std::env;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <gpio_pin>", args[0]);
        eprintln!("\nExample:");
        eprintln!("  {} 4    # Test PIR sensor on GPIO 4", args[0]);
        std::process::exit(1);
    }

    let pin_number: u8 = args[1]
        .parse()
        .context("GPIO pin must be valid integer (0-27)")?;

    println!("=== PIR Sensor Test ===");
    println!("Pin: GPIO {}", pin_number);
    println!("Timeout: 10 seconds");
    println!("\nEvents:");
    println!("  • PresenceDetected - when motion occurs");
    println!("  • NoPresenceSince - after 10s without motion");
    println!("\nPress Ctrl+C to exit\n");

    // Initialize with 10 second timeout
    let timeout = Duration::from_secs(10);
    let mut pir = PirSensorController::new(pin_number, "PIR Test", timeout)?;

    println!("Ready. Wave hand to trigger sensor.\n");

    loop {
        if let Some(event) = pir.check_motion() {
            println!("Event: {:?}", event);
        }

        sleep(Duration::from_millis(100)).await;
    }
}
