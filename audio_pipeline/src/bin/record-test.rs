use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::{WavSpec, WavWriter};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

/// Simple linear interpolation resampler
fn resample_linear(input: &[f32], input_rate: u32, output_rate: u32) -> Vec<f32> {
    if input_rate == output_rate {
        return input.to_vec();
    }

    let ratio = input_rate as f64 / output_rate as f64;
    let output_len = (input.len() as f64 / ratio).ceil() as usize;
    let mut output = Vec::with_capacity(output_len);

    for i in 0..output_len {
        let src_pos = i as f64 * ratio;
        let src_idx = src_pos.floor() as usize;
        let frac = src_pos - src_idx as f64;

        if src_idx + 1 < input.len() {
            let sample = input[src_idx] * (1.0 - frac) as f32 + input[src_idx + 1] * frac as f32;
            output.push(sample);
        } else if src_idx < input.len() {
            output.push(input[src_idx]);
        }
    }

    output
}

fn main() -> Result<()> {
    println!("=== Audio Recording Test (16kHz Mono - Vosk Format) ===");
    println!("This records audio in the same format that Vosk expects.");
    println!();

    // Configuration
    let duration_secs = 5;
    let output_file = "test_recording_16k.wav";
    let target_rate = 16000;

    println!("Configuration:");
    println!("  Duration: {} seconds", duration_secs);
    println!("  Output file: {}", output_file);
    println!("  Target format: {} Hz, mono", target_rate);
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
    println!("Will convert to: {} Hz, mono", target_rate);
    println!();
    println!("Recording... speak into the microphone!");
    println!();

    // Create WAV writer for 16kHz mono
    let spec = WavSpec {
        channels: 1,
        sample_rate: target_rate,
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
                    // Convert stereo to mono
                    let mono_data: Vec<f32> = if hw_channels == 2 {
                        data.chunks_exact(2)
                            .map(|frame| (frame[0] + frame[1]) / 2.0)
                            .collect()
                    } else {
                        data.to_vec()
                    };

                    // Resample to target rate
                    let resampled = resample_linear(&mono_data, hw_sample_rate, target_rate);

                    // Write samples
                    let mut writer = writer.lock().unwrap();
                    for sample in resampled {
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
                    // Convert to f32 and handle stereo->mono
                    let mono_f32: Vec<f32> = if hw_channels == 2 {
                        data.chunks_exact(2)
                            .map(|frame| ((frame[0] as f32 + frame[1] as f32) / 2.0) / 32768.0)
                            .collect()
                    } else {
                        data.iter().map(|&s| s as f32 / 32768.0).collect()
                    };

                    // Resample to target rate
                    let resampled = resample_linear(&mono_f32, hw_sample_rate, target_rate);

                    // Write samples
                    let mut writer = writer.lock().unwrap();
                    for sample in resampled {
                        let sample_i16 = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
                        let _ = writer.write_sample(sample_i16);
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
                    // Convert to f32 and handle stereo->mono
                    let mono_f32: Vec<f32> = if hw_channels == 2 {
                        data.chunks_exact(2)
                            .map(|frame| {
                                let s0 = (frame[0] as i32 - 32768) as f32 / 32768.0;
                                let s1 = (frame[1] as i32 - 32768) as f32 / 32768.0;
                                (s0 + s1) / 2.0
                            })
                            .collect()
                    } else {
                        data.iter()
                            .map(|&s| (s as i32 - 32768) as f32 / 32768.0)
                            .collect()
                    };

                    // Resample to target rate
                    let resampled = resample_linear(&mono_f32, hw_sample_rate, target_rate);

                    // Write samples
                    let mut writer = writer.lock().unwrap();
                    for sample in resampled {
                        let sample_i16 = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
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
    println!(
        "Format: {} Hz, mono, 16-bit (same as Vosk input)",
        target_rate
    );
    println!();
    println!("You can play it back with:");
    println!("  aplay {}", output_file);
    println!("or");
    println!("  ffplay {}", output_file);

    Ok(())
}
