//! LCD Display Test Binary
//!
//! Test LCD display by specifying I2C address via CLI arguments.
//! Tests basic operations, backlight control, and timed display functionality.

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
    info!("Press Ctrl+C to exit");

    // Initialize LCD controller
    let mut lcd = LcdController::new(address, "LCD Test")?;

    // ========================================================================
    // Test 1: Backlight Control
    // ========================================================================
    info!("\n--- Test 1: Backlight Control ---");

    info!("Turning backlight OFF...");
    lcd.backlight_off()?;
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    info!("Turning backlight ON...");
    lcd.backlight_on()?;
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // ========================================================================
    // Test 2: Basic Display
    // ========================================================================
    info!("\n--- Test 2: Basic Display ---");

    lcd.write_line(0, "Hello, World!")?;
    lcd.write_line(1, "LCD Working!")?;
    info!("Test messages displayed");
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // ========================================================================
    // Test 3: Clear Display
    // ========================================================================
    info!("\n--- Test 3: Clear Display ---");

    lcd.clear()?;
    info!("Display cleared");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // ========================================================================
    // Test 4: Backlight Off
    // ========================================================================
    info!("\n--- Test 4: Backlight Off ---");

    lcd.backlight_off()?;
    info!("Backlight turned off");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // ========================================================================
    // Test 5: Timed Display (New Feature)
    // ========================================================================
    info!("\n--- Test 5: Timed Display (New Feature) ---");
    info!("This test demonstrates the new 'display with duration' functionality:");
    info!("  • Turns backlight on");
    info!("  • Displays message");
    info!("  • Waits for specified duration");
    info!("  • Clears display and turns backlight off automatically");

    // Test with 5 second display
    lcd.display_with_duration("<(^-^)>", "I helped!", tokio::time::Duration::from_secs(5))
        .await?;

    info!("First timed display complete (5 seconds)");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Test with 3 second display
    lcd.display_with_duration(
        "(◕‿◕)",
        "Happy to help!",
        tokio::time::Duration::from_secs(3),
    )
    .await?;

    info!("Second timed display complete (3 seconds)");
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // ========================================================================
    // Test Complete
    // ========================================================================
    info!("\n=== All Tests Complete ===");
    info!("LCD is working correctly!");
    info!("Press Ctrl+C to exit.");

    // Keep running
    tokio::signal::ctrl_c().await?;

    // Clean up on exit
    lcd.clear()?;
    lcd.backlight_off()?;

    Ok(())
}
