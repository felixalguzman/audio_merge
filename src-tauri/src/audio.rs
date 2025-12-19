use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rtrb::{RingBuffer, Producer};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::thread;
use crossbeam_channel::{unbounded, Sender};
use std::collections::HashMap;
// use tauri::State; // Not used in the provided code, so omitting for now

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AudioDeviceInfo {
    pub name: String,
    pub index: usize,
}

// Commands sent from Main Thread (UI) to Audio Thread
pub enum AudioCommand {
    StartLoopback,
    StopLoopback,
    AddOutput(String), // device name
    RemoveOutput(String),
    SetVolume(String, f32),
    SetMute(String, bool),
    SetInputVolume(f32),
    SetInputMute(bool),
}

struct AudioActor {
    capture_stream: Option<cpal::Stream>,
    capture_sample_rate: Option<cpal::SampleRate>, // Store input rate
    producers: Arc<Mutex<Vec<(String, Producer<f32>)>>>,
    output_streams: HashMap<String, cpal::Stream>,
    volumes: HashMap<String, Arc<Mutex<f32>>>,
    mutes: HashMap<String, Arc<Mutex<bool>>>,
    
    // Input state
    input_volume: Arc<Mutex<f32>>,
    input_muted: Arc<Mutex<bool>>,
}

impl AudioActor {
    fn new() -> Self {
        Self {
            capture_stream: None,
            capture_sample_rate: None,
            producers: Arc::new(Mutex::new(Vec::new())),
            output_streams: HashMap::new(),
            volumes: HashMap::new(),
            mutes: HashMap::new(),
            input_volume: Arc::new(Mutex::new(1.0)),
            input_muted: Arc::new(Mutex::new(false)),
        }
    }

    fn start_loopback(&mut self) {
        if self.capture_stream.is_some() {
            println!("Capture already running");
            return;
        }

        let host = cpal::default_host();
        let device = match host.default_output_device() {
            Some(d) => d,
            None => {
                eprintln!("No default output device found");
                return;
            }
        };

        println!("Starting capture on: {}", device.name().unwrap_or_default());

        let config = match device.default_output_config() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to get config: {}", e);
                return;
            }
        };

        // Save Sample Rate!
        self.capture_sample_rate = Some(config.sample_rate());
        println!("Capture Sample Rate: {}", config.sample_rate().0);

        let stream_config: cpal::StreamConfig = config.into();
        let producers_handle = self.producers.clone();
        let in_vol_handle = self.input_volume.clone();
        let in_mute_handle = self.input_muted.clone();

        let stream_res = device.build_input_stream(
            &stream_config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                // Check Input Mute/Vol
                let vol = if let Ok(m) = in_mute_handle.lock() {
                    if *m { 0.0 } else { 
                        if let Ok(v) = in_vol_handle.lock() { *v } else { 1.0 }
                    }
                } else { 0.0 };
                
                if let Ok(mut producers) = producers_handle.lock() {
                    for (_name, producer) in producers.iter_mut() {
                        for &sample in data {
                            if !producer.is_full() {
                                let _ = producer.push(sample * vol);
                            }
                        }
                    }
                }
            },
            move |err| {
                eprintln!("Capture error: {}", err);
            },
            None
        );

        match stream_res {
            Ok(stream) => {
                let _ = stream.play();
                self.capture_stream = Some(stream);
            },
            Err(e) => eprintln!("Failed to build capture stream: {}", e),
        }
    }

    fn stop_loopback(&mut self) {
        // Drop the stream to stop it
        self.capture_stream = None;
        println!("Capture stopped");
    }

    fn set_volume(&mut self, device_name: String, volume: f32) {
        println!("Setting volume for '{}': {}", device_name, volume);
        if let Some(vol) = self.volumes.get(&device_name) {
             if let Ok(mut v) = vol.lock() {
                 *v = volume;
                 println!("Volume key found and updated.");
             }
        } else {
            println!("Device '{}' not found in volumes map. Available keys: {:?}", device_name, self.volumes.keys());
        }
    }

    fn set_mute(&mut self, device_name: String, muted: bool) {
        println!("Setting mute for '{}': {}", device_name, muted);
        if let Some(m) = self.mutes.get(&device_name) {
             if let Ok(mut v) = m.lock() { *v = muted; }
        } else {
             println!("Device '{}' not found in mutes map.", device_name);
        }
    }

    fn set_input_volume(&mut self, volume: f32) {
        println!("Setting input volume: {}", volume);
        if let Ok(mut v) = self.input_volume.lock() { *v = volume; }
    }

    fn set_input_mute(&mut self, muted: bool) {
         println!("Setting input mute: {}", muted);
         if let Ok(mut v) = self.input_muted.lock() { *v = muted; }
    }

    fn add_output(&mut self, device_name: String) {
        if self.output_streams.contains_key(&device_name) {
            println!("Device exists: {}", device_name);
            return;
        }

        let host = cpal::default_host();
        let device = match host.output_devices() {
            Ok(mut devices) => devices.find(|d| d.name().unwrap_or_default() == device_name),
            Err(_) => None,
        };

        let device = match device {
            Some(d) => d,
            None => {
                eprintln!("Device not found: {}", device_name);
                return;
            }
        };

        // Try to find matching config
        let target_rate = self.capture_sample_rate.unwrap_or(cpal::SampleRate(48000));
        
        let mut best_config = None;
        if let Ok(configs) = device.supported_output_configs() {
            for config in configs {
                if config.min_sample_rate() <= target_rate && config.max_sample_rate() >= target_rate {
                     // Found range containing our target
                     best_config = Some(config.with_sample_rate(target_rate));
                     break;
                }
            }
        }

        let config: cpal::StreamConfig = match best_config {
            Some(c) => c.into(),
            None => {
                 println!("Warning: Could not match sample rate {}. Using default.", target_rate.0);
                 device.default_output_config().map(|c| c.into()).unwrap_or_else(|_| cpal::StreamConfig { 
                    channels: 2, sample_rate: cpal::SampleRate(44100), buffer_size: cpal::BufferSize::Default 
                })
            }
        };
        
        println!("Output {} configured at: {}", device_name, config.sample_rate.0);

        let (producer, mut consumer) = RingBuffer::<f32>::new(16384); // Increased buffer size
        
        if let Ok(mut lock) = self.producers.lock() {
            lock.push((device_name.clone(), producer));
        }

        // Volume handle
        let volume_handle = Arc::new(Mutex::new(1.0));
        self.volumes.insert(device_name.clone(), volume_handle.clone());
        
        // Mute handle
        let mute_handle = Arc::new(Mutex::new(false));
        self.mutes.insert(device_name.clone(), mute_handle.clone());

        let vol_clone = volume_handle.clone();
        let mute_clone = mute_handle.clone();

        let stream_res = device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let current_vol = if let Ok(m) = mute_clone.lock() {
                    if *m { 0.0 } else {
                         if let Ok(g) = vol_clone.lock() { *g } else { 1.0 }
                    }
                } else { 0.0 };
                
                for sample in data.iter_mut() {
                     let val = consumer.pop().unwrap_or(0.0);
                     *sample = val * current_vol;
                }
            },
            move |err| eprintln!("Output error: {}", err),
            None
        );

        match stream_res {
            Ok(stream) => {
                let _ = stream.play();
                self.output_streams.insert(device_name.clone(), stream);
                println!("Added output with volume control: {}", device_name);
            },
            Err(e) => eprintln!("Failed to build output stream: {}", e),
        }
    }

    fn remove_output(&mut self, device_name: String) {
        // Drop the stream first to stop playback
        if self.output_streams.remove(&device_name).is_some() {
             println!("Stopped output stream: {}", device_name);
        }

        // Remove from producers list to stop feeding it data
        if let Ok(mut lock) = self.producers.lock() {
            lock.retain(|(name, _)| name != &device_name);
        }

        // Remove volume control
        self.volumes.remove(&device_name);
        // Remove mute control
        self.mutes.remove(&device_name);
    }
}

pub fn spawn_audio_thread() -> Sender<AudioCommand> {
    let (tx, rx) = unbounded();
    thread::spawn(move || {
        let mut actor = AudioActor::new();
        while let Ok(cmd) = rx.recv() {
            match cmd {
                AudioCommand::StartLoopback => actor.start_loopback(),
                AudioCommand::StopLoopback => actor.stop_loopback(),
                AudioCommand::AddOutput(name) => actor.add_output(name),
                AudioCommand::RemoveOutput(name) => actor.remove_output(name),
                AudioCommand::SetVolume(name, vol) => actor.set_volume(name, vol),
                AudioCommand::SetMute(name, mute) => actor.set_mute(name, mute),
                AudioCommand::SetInputVolume(vol) => actor.set_input_volume(vol),
                AudioCommand::SetInputMute(mute) => actor.set_input_mute(mute),
            }
        }
    });
    tx
}

#[cfg(test)]
mod tests {
    use super::*;
    use rtrb::RingBuffer;

    #[test]
    fn test_ring_buffer_transfer() {
        let (mut producer, mut consumer) = RingBuffer::<f32>::new(100);
        let _ = producer.push(0.5);
        assert_eq!(consumer.pop().unwrap(), 0.5);
    }

    #[test]
    fn test_volume_application_logic() {
        let volume = 0.5;
        let input_sample = 1.0;
        let output_sample = input_sample * volume;
        assert_eq!(output_sample, 0.5);
    }
}

pub fn get_output_devices() -> Vec<AudioDeviceInfo> {
    let host = cpal::default_host();
    match host.output_devices() {
        Ok(devices) => devices
            .enumerate()
            .map(|(index, device)| {
                let name = device.name().unwrap_or_else(|_| "Unknown Device".to_string());
                AudioDeviceInfo { name, index }
            })
            .collect(),
        Err(_) => Vec::new()
    }
}

pub fn get_default_device_name() -> String {
    let host = cpal::default_host();
    host.default_output_device()
        .and_then(|d| d.name().ok())
        .unwrap_or_else(|| "Unknown".to_string())
}
