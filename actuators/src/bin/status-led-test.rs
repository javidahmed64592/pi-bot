//! Status LED Test Binary
//!
//! Test status LEDs by specifying GPIO pin numbers for all 4 status LEDs.
//! This allows testing that status LEDs are wired correctly.

use anyhow::{Context, Result};
use log::info;
use std::env;
use std::time::{Duration, Instant};

// ============================================================================
// Pattern Demo Functions
// ============================================================================

/// Demonstrate breathing pattern (fade in/out)
async fn demo_breathing(led: &mut actuators::StatusLedController, duration: Duration) {
    let start = Instant::now();
    let cycle_duration = Duration::from_millis(2000); // 2 seconds per breath cycle

    while start.elapsed() < duration {
        let elapsed_in_cycle = start.elapsed().as_millis() % cycle_duration.as_millis();
        let progress = elapsed_in_cycle as f32 / cycle_duration.as_millis() as f32;

        // Sine wave for smooth breathing (0.2 to 1.0 brightness)
        let brightness = 0.2 + 0.8 * (progress * std::f32::consts::PI * 2.0).sin().abs();

        led.set_brightness(brightness);
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
}

/// Demonstrate flashing pattern (fast on/off)
async fn demo_flashing(led: &mut actuators::StatusLedController, duration: Duration) {
    let start = Instant::now();
    let flash_interval = Duration::from_millis(250); // 250ms on, 250ms off

    while start.elapsed() < duration {
        led.set_brightness(1.0);
        tokio::time::sleep(flash_interval).await;
        led.set_brightness(0.0);
        tokio::time::sleep(flash_interval).await;
    }
}

// ============================================================================
// Main Test
// ============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging (use RUST_LOG=debug for verbose output)
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .init();

    // Parse command-line arguments
    let args: Vec<String> = env::args().collect();

    if args.len() != 5 {
        eprintln!(
            "Usage: {} <green1_pin> <green2_pin> <red1_pin> <red2_pin>",
            args[0]
        );
        eprintln!("\nExample:");
        eprintln!("  {} 18 25 23 24    # Test all 4 status LEDs", args[0]);
        eprintln!("\nFrom config.yaml:");
        eprintln!("  Green LEDs: GPIO 18, 25 (active state indicators)");
        eprintln!("  Red LEDs:   GPIO 23, 24 (idle/error state indicators)");
        std::process::exit(1);
    }

    let green1_pin: u8 = args[1]
        .parse()
        .context("Green1 pin number must be a valid integer (0-27)")?;
    let green2_pin: u8 = args[2]
        .parse()
        .context("Green2 pin number must be a valid integer (0-27)")?;
    let red1_pin: u8 = args[3]
        .parse()
        .context("Red1 pin number must be a valid integer (0-27)")?;
    let red2_pin: u8 = args[4]
        .parse()
        .context("Red2 pin number must be a valid integer (0-27)")?;

    info!("=== Status LED Test ===");
    info!("Testing 4 status LEDs:");
    info!("  Green LED 1: GPIO {}", green1_pin);
    info!("  Green LED 2: GPIO {}", green2_pin);
    info!("  Red LED 1:   GPIO {}", red1_pin);
    info!("  Red LED 2:   GPIO {}", red2_pin);
    info!("\nThis test will demonstrate:");
    info!("  1. Individual LED control (each LED separately)");
    info!("  2. Brightness levels (25%, 50%, 75%, 100%)");
    info!("  3. Group control (all on, all off)");
    info!("  4. Pattern demos (breathing, flashing)");
    info!("\nPress Ctrl+C to exit early\n");

    // Initialize all LEDs
    let mut green1 = actuators::StatusLedController::new(green1_pin, "Green LED 1")?;
    let mut green2 = actuators::StatusLedController::new(green2_pin, "Green LED 2")?;
    let mut red1 = actuators::StatusLedController::new(red1_pin, "Red LED 1")?;
    let mut red2 = actuators::StatusLedController::new(red2_pin, "Red LED 2")?;

    // Ensure all LEDs start off
    green1.turn_off();
    green2.turn_off();
    red1.turn_off();
    red2.turn_off();
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Test 1: Individual LED control
    info!("\n=== Test 1: Individual LED Control ===");

    info!("\n[1/4] Green LED 1 only");
    green1.turn_on();
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    green1.turn_off();
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    info!("[2/4] Green LED 2 only");
    green2.turn_on();
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    green2.turn_off();
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    info!("[3/4] Red LED 1 only");
    red1.turn_on();
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    red1.turn_off();
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    info!("[4/4] Red LED 2 only");
    red2.turn_on();
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    red2.turn_off();
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Test 2: Brightness levels
    info!("\n=== Test 2: Brightness Levels (Green LED 1) ===");

    for brightness in [0.25, 0.5, 0.75, 1.0] {
        info!("\nBrightness: {:.0}%", brightness * 100.0);
        green1.set_brightness(brightness);
        tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
    }
    green1.turn_off();
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Test 3: Group control
    info!("\n=== Test 3: Group Control ===");

    info!("\n[1/4] All LEDs on (full brightness)");
    green1.turn_on();
    green2.turn_on();
    red1.turn_on();
    red2.turn_on();
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    info!("[2/4] Green pair only");
    red1.turn_off();
    red2.turn_off();
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    info!("[3/4] Red pair only");
    green1.turn_off();
    green2.turn_off();
    red1.turn_on();
    red2.turn_on();
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    info!("[4/4] All LEDs off");
    red1.turn_off();
    red2.turn_off();
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Test 4: Pattern demos
    info!("\n=== Test 4: Pattern Demos ===");

    info!("\n[1/3] Breathing pattern (Green pair, 6 seconds)");
    tokio::join!(
        demo_breathing(&mut green1, Duration::from_secs(6)),
        demo_breathing(&mut green2, Duration::from_secs(6))
    );
    green1.turn_off();
    green2.turn_off();
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    info!("[2/3] Flashing pattern (Red pair, 4 seconds)");
    tokio::join!(
        demo_flashing(&mut red1, Duration::from_secs(4)),
        demo_flashing(&mut red2, Duration::from_secs(4))
    );
    red1.turn_off();
    red2.turn_off();
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    info!("[3/3] Solid pattern (All LEDs, 3 seconds)");
    green1.turn_on();
    green2.turn_on();
    red1.turn_on();
    red2.turn_on();
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Final cleanup
    green1.turn_off();
    green2.turn_off();
    red1.turn_off();
    red2.turn_off();

    info!("\n=== Test Complete ===");
    info!("✓ All LED tests passed!");
    info!("\nIf you saw all the LEDs and patterns correctly, your status LEDs are wired properly:");
    info!("  • Green LED 1 on GPIO {}", green1_pin);
    info!("  • Green LED 2 on GPIO {}", green2_pin);
    info!("  • Red LED 1 on GPIO {}", red1_pin);
    info!("  • Red LED 2 on GPIO {}", red2_pin);
    info!(
        "\nNote: If LEDs don't light up, check wiring and verify pin assignments in config.yaml\n"
    );

    Ok(())
}
