use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::{WavSpec, WavWriter};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

fn main() -> Result<()> {
    println!("=== Audio Recording Test ===");
    println!("This will record audio from your microphone and save it to a WAV file.");
    println!();

    // Configuration
    let duration_secs = 5;
    let output_file = "test_recording.wav";

    println!("Configuration:");
    println!("  Duration: {} seconds", duration_secs);
    println!("  Output file: {}", output_file);
    println!();

    // Get audio device
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| anyhow::anyhow!("No input device found"))?;

    println!("Using audio device: {}", device.name()?);

    // Get hardware configuration
    let hw_config = device.default_input_config()?;
    let hw_sample_rate = hw_config.sample_rate().0;
    let hw_channels = hw_config.channels();
    let hw_format = hw_config.sample_format();

    println!(
        "Hardware config: {} Hz, {} channel(s), {:?}",
        hw_sample_rate, hw_channels, hw_format
    );
    println!();
    println!("Recording... speak into the microphone!");
    println!();

    // Create WAV writer
    let spec = WavSpec {
        channels: hw_channels,
        sample_rate: hw_sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let writer = Arc::new(Mutex::new(WavWriter::create(output_file, spec)?));

    // Channel for stopping the stream
    let (stop_tx, stop_rx) = mpsc::channel::<()>();

    // Spawn thread to stop recording after duration
    let stop_tx_clone = stop_tx.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(duration_secs));
        let _ = stop_tx_clone.send(());
    });

    // Build input stream based on format
    let stream = match hw_format {
        cpal::SampleFormat::F32 => {
            let writer = Arc::clone(&writer);
            device.build_input_stream(
                &hw_config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    let mut writer = writer.lock().unwrap();
                    for &sample in data {
                        // Convert f32 to i16
                        let sample_i16 = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
                        let _ = writer.write_sample(sample_i16);
                    }
                },
                |err| eprintln!("Audio stream error: {}", err),
                None,
            )?
        }
        cpal::SampleFormat::I16 => {
            let writer = Arc::clone(&writer);
            device.build_input_stream(
                &hw_config.into(),
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    let mut writer = writer.lock().unwrap();
                    for &sample in data {
                        let _ = writer.write_sample(sample);
                    }
                },
                |err| eprintln!("Audio stream error: {}", err),
                None,
            )?
        }
        cpal::SampleFormat::U16 => {
            let writer = Arc::clone(&writer);
            device.build_input_stream(
                &hw_config.into(),
                move |data: &[u16], _: &cpal::InputCallbackInfo| {
                    let mut writer = writer.lock().unwrap();
                    for &sample in data {
                        // Convert u16 to i16
                        let sample_i16 = (sample as i32 - 32768) as i16;
                        let _ = writer.write_sample(sample_i16);
                    }
                },
                |err| eprintln!("Audio stream error: {}", err),
                None,
            )?
        }
        _ => return Err(anyhow::anyhow!("Unsupported sample format")),
    };

    // Start recording
    stream.play()?;

    // Wait for stop signal
    let _ = stop_rx.recv();

    // Stop stream
    drop(stream);

    // Finalize WAV file
    let writer =
        Arc::try_unwrap(writer).map_err(|_| anyhow::anyhow!("Failed to finalize writer"))?;
    writer.into_inner().unwrap().finalize()?;

    println!();
    println!("✓ Recording complete!");
    println!();
    println!("Saved to: {}", output_file);
    println!();
    println!("You can play it back with:");
    println!("  aplay {}", output_file);
    println!("or");
    println!("  ffplay {}", output_file);

    Ok(())
}
