//! RGB LED Test Binary
//!
//! Test RGB LED by specifying GPIO pin numbers for red, green, and blue channels.
//! This allows testing that RGB LEDs are wired correctly and demonstrates color mixing.

use anyhow::{Context, Result};
use bot_core::RgbColor;
use log::info;
use std::env;
use std::time::{Duration, Instant};

// ============================================================================
// Pattern Demo Functions
// ============================================================================

/// Demonstrate breathing pattern (fade in/out)
async fn demo_breathing(
    led: &mut actuators::RgbLedController,
    color: RgbColor,
    duration: Duration,
) {
    let start = Instant::now();
    let cycle_duration = Duration::from_millis(2000); // 2 seconds per breath cycle

    while start.elapsed() < duration {
        let elapsed_in_cycle = start.elapsed().as_millis() % cycle_duration.as_millis();
        let progress = elapsed_in_cycle as f32 / cycle_duration.as_millis() as f32;

        // Sine wave for smooth breathing (0.2 to 1.0 brightness)
        let brightness = 0.2 + 0.8 * (progress * std::f32::consts::PI * 2.0).sin().abs();

        led.set_brightness(brightness);
        led.set_color(color);
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
}

/// Demonstrate pulse pattern (quick flash)
async fn demo_pulse(led: &mut actuators::RgbLedController, color: RgbColor, count: u8) {
    for _ in 0..count {
        // Quick ramp up
        for i in 0..10 {
            led.set_brightness(i as f32 / 10.0);
            led.set_color(color);
            tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
        }
        // Quick ramp down
        for i in (0..10).rev() {
            led.set_brightness(i as f32 / 10.0);
            led.set_color(color);
            tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }
}

/// Demonstrate gradient pattern (transition between colors)
async fn demo_gradient(
    led: &mut actuators::RgbLedController,
    colors: &[RgbColor],
    duration: Duration,
) {
    if colors.len() < 2 {
        return;
    }

    // Ensure brightness is at maximum for visible colors
    led.set_brightness(1.0);

    let start = Instant::now();
    let segment_duration = duration.as_millis() / (colors.len() - 1) as u128;

    while start.elapsed() < duration {
        let elapsed = start.elapsed().as_millis();
        let segment = (elapsed / segment_duration) as usize;

        if segment >= colors.len() - 1 {
            break;
        }

        let progress = (elapsed % segment_duration) as f32 / segment_duration as f32;
        let color = colors[segment].lerp(&colors[segment + 1], progress);

        led.set_color(color);
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
}

/// Demonstrate rainbow pattern (cycle through hue)
async fn demo_rainbow(led: &mut actuators::RgbLedController, duration: Duration) {
    // Ensure brightness is at maximum for visible colors
    led.set_brightness(1.0);

    let start = Instant::now();
    let colors = [
        RgbColor::RED,
        RgbColor::ORANGE,
        RgbColor::YELLOW,
        RgbColor::GREEN,
        RgbColor::CYAN,
        RgbColor::BLUE,
        RgbColor::PURPLE,
    ];

    while start.elapsed() < duration {
        let elapsed = start.elapsed().as_millis();
        let total_ms = duration.as_millis();
        let progress = (elapsed % total_ms) as f32 / total_ms as f32;
        let index = (progress * colors.len() as f32) as usize % colors.len();
        let next_index = (index + 1) % colors.len();
        let segment_progress = (progress * colors.len() as f32) % 1.0;

        let color = colors[index].lerp(&colors[next_index], segment_progress);
        led.set_color(color);
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
}

/// Demonstrate color cycle pattern (discrete color changes)
async fn demo_color_cycle(
    led: &mut actuators::RgbLedController,
    colors: &[RgbColor],
    duration: Duration,
) {
    // Ensure brightness is at maximum for visible colors
    led.set_brightness(1.0);

    let start = Instant::now();
    let color_duration = duration.as_millis() / colors.len() as u128;

    while start.elapsed() < duration {
        let elapsed = start.elapsed().as_millis();
        let index = ((elapsed / color_duration) as usize) % colors.len();
        led.set_color(colors[index]);
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
}

/// Demonstrate error blink pattern (rapid red flashes)
async fn demo_blink_error(led: &mut actuators::RgbLedController, times: u8) {
    let original_color = led.get_color();
    let original_brightness = led.get_brightness();

    for i in 0..times {
        // Fast blink pattern: red at full brightness
        led.set_brightness(1.0);
        led.set_color(RgbColor::RED);
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        led.set_color(RgbColor::OFF);

        // Short pause between blinks except on last one
        if i < times - 1 {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }

    // Restore original color and brightness after blinking
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    led.set_brightness(original_brightness);
    led.set_color(original_color);
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

    if args.len() != 4 {
        eprintln!("Usage: {} <red_pin> <green_pin> <blue_pin>", args[0]);
        eprintln!("\nExample:");
        eprintln!(
            "  {} 23 24 25    # Test RGB LED with R=GPIO23, G=GPIO24, B=GPIO25",
            args[0]
        );
        eprintln!("\nFrom config.yaml:");
        eprintln!("  {} 23 24 25    # System RGB LED", args[0]);
        std::process::exit(1);
    }

    let red_pin: u8 = args[1]
        .parse()
        .context("Red pin number must be a valid integer (0-27)")?;
    let green_pin: u8 = args[2]
        .parse()
        .context("Green pin number must be a valid integer (0-27)")?;
    let blue_pin: u8 = args[3]
        .parse()
        .context("Blue pin number must be a valid integer (0-27)")?;

    info!("=== RGB LED Test ===");
    info!("Testing RGB LED:");
    info!("  Red channel:   GPIO {}", red_pin);
    info!("  Green channel: GPIO {}", green_pin);
    info!("  Blue channel:  GPIO {}", blue_pin);
    info!("\nThis test will demonstrate:");
    info!("  1. Individual color channels (Red, Green, Blue)");
    info!("  2. Predefined colors (Green, Blue, Orange, Red)");
    info!("  3. Brightness levels (25%, 50%, 75%, 100%)");
    info!("  4. Mixed colors (Yellow, Cyan, Purple, White)");
    info!("  5. Error blink pattern");
    info!("  6. LED patterns (Solid, Breathing, Pulse, Gradient, Rainbow, Color Cycle)");
    info!("\nPress Ctrl+C to exit early\n");

    // Initialize RGB LED
    let mut led = actuators::RgbLedController::new(red_pin, green_pin, blue_pin, "RGB LED Test")?;

    // Ensure LED starts off
    led.turn_off();
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Test 1: Individual color channels
    info!("\n=== Test 1: Individual Color Channels ===");

    info!("\n[1/3] Red channel only (full)");
    led.set_color(RgbColor::RED);
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    info!("[2/3] Green channel only (full)");
    led.set_color(RgbColor::GREEN);
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    info!("[3/3] Blue channel only (full)");
    led.set_color(RgbColor::BLUE);
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    led.turn_off();
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Test 2: Predefined colors
    info!("\n=== Test 2: Predefined Colors ===");

    info!("\n[1/4] Green (normal operation)");
    led.set_color(RgbColor::GREEN);
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    info!("[2/4] Blue (motion detected)");
    led.set_color(RgbColor::BLUE);
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    info!("[3/4] Orange (system busy)");
    led.set_color(RgbColor::ORANGE);
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    info!("[4/4] Red (error)");
    led.set_color(RgbColor::RED);
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    led.turn_off();
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Test 3: Brightness levels
    info!("\n=== Test 3: Brightness Levels (Green Color) ===");
    led.set_color(RgbColor::GREEN);

    for brightness in [0.25, 0.5, 0.75, 1.0] {
        info!("\nBrightness: {:.0}%", brightness * 100.0);
        led.set_brightness(brightness);
        tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
    }

    led.turn_off();
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Test 4: Mixed colors
    info!("\n=== Test 4: Mixed Colors ===");

    info!("\n[1/5] Yellow (Red + Green)");
    led.set_color(RgbColor::YELLOW);
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    info!("[2/5] Cyan (Green + Blue)");
    led.set_color(RgbColor::CYAN);
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    info!("[3/5] Magenta (Red + Blue)");
    led.set_color(RgbColor::PURPLE);
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    info!("[4/5] White (All channels)");
    led.set_color(RgbColor::WHITE);
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    info!("[5/5] Dim white (50% brightness)");
    led.set_color(RgbColor::WHITE);
    led.set_brightness(0.5);
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    led.turn_off();
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Test 5: Error blink pattern
    info!("\n=== Test 5: Error Blink Pattern ===");
    info!("\nBlinking red 3 times...");
    led.set_color(RgbColor::GREEN);
    led.set_brightness(1.0);
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    demo_blink_error(&mut led, 3).await;
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    led.turn_off();
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Test 6: LED Patterns
    info!("\n=== Test 6: LED Patterns ===");

    info!("\n[1/6] Solid pattern (Green, 3 seconds)");
    led.set_brightness(1.0);
    led.set_color(RgbColor::GREEN);
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    info!("[2/6] Breathing pattern (Blue, 6 seconds)");
    demo_breathing(&mut led, RgbColor::BLUE, Duration::from_secs(6)).await;

    info!("[3/6] Pulse pattern (Orange, 3 pulses)");
    demo_pulse(&mut led, RgbColor::ORANGE, 3).await;

    info!("[4/6] Gradient pattern (Red → Yellow → Green, 6 seconds)");
    demo_gradient(
        &mut led,
        &[RgbColor::RED, RgbColor::YELLOW, RgbColor::GREEN],
        Duration::from_secs(6),
    )
    .await;

    info!("[5/6] Rainbow pattern (8 seconds)");
    demo_rainbow(&mut led, Duration::from_secs(8)).await;

    info!("[6/6] Color cycle pattern (Red → Green → Blue → Purple, 8 seconds)");
    demo_color_cycle(
        &mut led,
        &[
            RgbColor::RED,
            RgbColor::GREEN,
            RgbColor::BLUE,
            RgbColor::PURPLE,
        ],
        Duration::from_secs(8),
    )
    .await;

    // Final cleanup
    led.turn_off();

    info!("\n=== Test Complete ===");
    info!("✓ All color and pattern tests passed!");
    info!("\nIf you saw all the colors and patterns correctly, your RGB LED is wired properly:");
    info!("  • Red channel on GPIO {}", red_pin);
    info!("  • Green channel on GPIO {}", green_pin);
    info!("  • Blue channel on GPIO {}", blue_pin);
    info!("\nNote: If colors appear wrong (e.g., red shows as blue),");
    info!("      check your wiring and verify pin assignments in config.yaml\n");

    Ok(())
}
