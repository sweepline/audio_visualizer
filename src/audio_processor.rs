use crate::{fft_buffer::FFTDimensions, TextureHandle};
use core::f32::consts::PI;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    BufferSize, Stream, StreamConfig, SupportedBufferSize,
};
use ringbuf::{Consumer, HeapRb};
use rustfft::{num_complex::Complex32, FftPlanner};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

pub struct AudioProcessor {
    // wave_texture: TextureHandle,
    fft_texture: TextureHandle,
    // beat_texture: TextureHandle,
    fft_thread: JoinHandle<()>,
    dimensions: FFTDimensions,
    input_stream: Stream,
    stream_config: StreamConfig,
}

impl AudioProcessor {
    pub fn new(dimensions: FFTDimensions) -> Self {
        // Setup CPAL for recording
        let host = cpal::default_host();
        let device = host.default_input_device().unwrap();

        let mut config = device
            .default_input_config()
            .expect("Failed to get default input config")
            .config();
        let sample_rate = config.sample_rate.0 as f32;
        let supported = device.default_input_config().unwrap();
        // TODO: Somethingsfucky about the buffersize
        let _data_size = supported.sample_format().sample_size();
        let fft_size = dimensions.fft_size as u32;

        let sr_ms = sample_rate / 1_000.;
        let sr_us = sr_ms / 1_000.;
        let fft_delay_us = (fft_size as f32 / sr_us).round() as u128;

        let bz = if let SupportedBufferSize::Range { .. } = supported.buffer_size() {
            BufferSize::Fixed(fft_size)
        } else {
            BufferSize::Default
        };
        // TODO: 2 Channels, the data will be interleaved [L, R, L, R]
        // I think we just combine the data v = 0.5 * (left_v + right_v)
        // But it requires a larget buffer and so on..
        config.channels = 1;
        config.buffer_size = bz;
        // config.buffer_size = BufferSize::Fixed(FrameCount)
        let channels = config.channels as usize;

        // Ring buffer for communication between CPAL and fft.
        let ring_buffer = HeapRb::<f32>::new(dimensions.ring_size());
        let (mut producer, consumer) = ring_buffer.split();
        for _ in 0..dimensions.ring_size() {
            producer.push(0.).unwrap();
        }

        // Set up CPAL listening
        fn err_fn(err: cpal::StreamError) {
            eprintln!("an error occurred on stream: {}", err);
        }
        let input_data_fn = move |data: &[f32], _: &cpal::InputCallbackInfo| {
            let mut output_fell_behind = false;
            for &sample in data {
                if producer.push(sample).is_err() {
                    output_fell_behind = true;
                }
            }
            if output_fell_behind {
                eprintln!("Output stream fell behind: try increasing latency");
            }
        };

        let input_stream = device
            .build_input_stream(&config, input_data_fn, err_fn, None)
            .unwrap();

        // Better performance with Arc<[Atomic]> instead of Arc<Mutex>
        let fft_texture: TextureHandle = Arc::new(Mutex::new(vec![0.; dimensions.texture_size()]));
        let thread_fft_tex = fft_texture.clone();
        let fft_thread = thread::spawn(move || {
            fft_analysis(consumer, thread_fft_tex, config.sample_rate, dimensions);
        });

        Self {
            fft_texture,
            fft_thread,
            input_stream,
            dimensions,
            stream_config: config,
        }
    }

    pub fn fft_texture(&self) -> TextureHandle {
        self.fft_texture.clone()
    }
}

pub fn blackman_single(sample: f32, n: f32, len: f32) -> f32 {
    let a0 = (1. - 0.16) / 2.;
    let a1 = 0.5;
    let a2 = 0.16 / 2.;
    let w = a0 - a1 * f32::cos((2. * PI * n) / len) + a2 * f32::cos((4. * PI * n) / len);
    sample * w
}

pub fn hann_single(sample: f32, i: usize, samples_len: usize) -> f32 {
    let samples_len_f32 = samples_len as f32;
    let two_pi_i = 2. * PI * i as f32;
    let cosine = f32::cos(two_pi_i / samples_len_f32);
    let multiplier = 0.5 * (1. - cosine);
    multiplier * sample
}

fn fft_analysis(
    mut consumer: Consumer<f32, Arc<HeapRb<f32>>>,
    texture_handle: Arc<Mutex<Vec<f32>>>,
    cpal::SampleRate(sample_rate): cpal::SampleRate,
    dimensions: FFTDimensions,
) {
    let fft_size = dimensions.fft_size;
    let sr_ms = sample_rate as f32 / 1_000.;
    let sr_us = sr_ms / 1_000.;
    let fft_delay_us = (fft_size as f32 / sr_us).round() as u128;
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);
    let mut scratch = vec![Complex32::default(); fft.get_inplace_scratch_len()];

    let mut amplitudes: Vec<f32> = vec![0.; fft_size / 2];
    let mut fft_buf: Vec<Complex32> = vec![Complex32::default(); fft_size];
    let mut timer = Instant::now();

    loop {
        // START FFT
        let elapsed = timer.elapsed().as_micros();
        if elapsed > fft_delay_us as u128 {
            // TODO: do something about time drift (fix your timestep).
            timer = Instant::now();
            // Time elapsed in microseconds * samples per microseconds
            // let exact_samples = elapsed as f32 * sr_us;
            // let time_drift = elapsed - fft_delay_us;

            let mut input_fell_behind = false;
            for i in 0..fft_size {
                let x = match consumer.pop() {
                    Some(s) => s,
                    None => {
                        input_fell_behind = true;
                        0.
                    }
                };
                // Apply windowing function to the input
                fft_buf[i] = Complex32::new(blackman_single(x, i as f32, fft_size as f32), 0.);
            }

            if input_fell_behind {
                eprintln!("Input stream fell behind: try increasing latency");
            }

            fft.process_with_scratch(&mut fft_buf, &mut scratch);

            // let _bin_freq = sample_rate / fft_size as f32;

            let Ok(mut texture) = texture_handle.lock() else {
                panic!("TEXTURE MUTEX FFT SIDE");
            };

            let texture_width = dimensions.texture_width() as usize;
            // TODO: Maybe move this out of this loop and to the main thread?
            // The buffer has the last TEXTURE_HEIGHT fft runs.
            // With the first TEXTURE_WIDTH elements being the newest run.
            // So rotate the elements back one TEXTURE_WIDTH and write the
            // new run to the buffer at the front.
            texture.rotate_right(texture_width);
            texture[..texture_width].copy_from_slice(&vec![0.; texture_width]);

            let freq_amp = fft_buf.iter().take(texture_width).enumerate();
            for (i, amp) in freq_amp {
                let amp = amp / fft_size as f32;
                let amp_prev = amplitudes[i];
                let amp =
                    dimensions.smoothing * amp_prev + (1. - dimensions.smoothing) * amp.norm();
                amplitudes[i] = amp;

                if i < texture_width {
                    texture[i] += amp;
                }
            }
            // TODO Move to config.
            const DB_LO: f32 = -100.;
            const DB_HI: f32 = -10.;
            for amp in texture.iter_mut().take(texture_width) {
                let db = 20. * f32::log10(*amp);
                let db = db.clamp(DB_LO, DB_HI);
                let normalized = (db - DB_LO) / (DB_HI - DB_LO);
                *amp = normalized;
            }

            // Done with the texture so drop it so the rendering can use it.
            drop(texture);

            // let max_peak = freq_amp.iter().max_by_key(|&(_, c)| *c as u32);
            // let min_peak = freq_amp.iter().min_by_key(|&(_, c)| *c as u32);
            // let min_freq = freq_amp.iter().min_by_key(|&(f, _)| *f as u32).unwrap();
            // let max_freq = freq_amp.iter().max_by_key(|&(f, _)| *f as u32).unwrap();
            const EARLY_WAKE_US: u128 = 2000; // Because we want to stop sleeping a little before.
            let remaining = fft_delay_us - timer.elapsed().as_micros() - EARLY_WAKE_US;
            spin_sleep::sleep(Duration::from_micros(remaining as u64));
        }
    }
}
