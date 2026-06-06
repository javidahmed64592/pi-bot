//! LCD Display Test Binary
//!
//! Test LCD display by specifying I2C address via CLI arguments.

use actuators::LcdController;
use anyhow::{Context, Result};
use log::info;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging (use RUST_LOG=debug for verbose output)
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .init();

    // Parse command-line arguments
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <i2c_address_hex>", args[0]);
        eprintln!("\nExample:");
        eprintln!("  {} 0x27    # Test LCD at I2C address 0x27", args[0]);
        std::process::exit(1);
    }

    // Parse hex address
    let address_str = args[1].trim_start_matches("0x");
    let address = u8::from_str_radix(address_str, 16)
        .context("I2C address must be a valid hex value (e.g., 0x27)")?;

    info!("=== LCD Display Test ===");
    info!("Testing LCD at I2C address: 0x{:02X}", address);
    info!("This test will:");
    info!("  • Initialize the I2C LCD display");
    info!("  • Display test messages on both rows");
    info!("  • Test backlight on/off");
    info!("Press Ctrl+C to exit");

    // Initialize LCD controller
    let mut lcd = LcdController::new(address, "LCD Test")?;

    // Display test messages
    lcd.write_line(0, "Hello, World!")?;
    lcd.write_line(1, "LCD Working!")?;

    info!("Test messages displayed.");
    info!("Backlight will blink in 3 seconds...");

    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Test backlight
    for i in 1..=3 {
        info!("Blink {}/3 - Backlight OFF", i);
        lcd.backlight_off()?;
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        info!("Blink {}/3 - Backlight ON", i);
        lcd.backlight_on()?;
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    info!("Test complete! LCD is working correctly.");
    info!("Press Ctrl+C to exit.");

    // Keep running
    tokio::signal::ctrl_c().await?;

    // Clear on exit
    lcd.clear()?;
    lcd.backlight_off()?;

    Ok(())
}
