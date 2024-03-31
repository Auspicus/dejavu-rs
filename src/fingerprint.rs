use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::SystemTime,
};

use minimp3::Frame;
use rayon::prelude::*;
use rustfft::{num_complex::Complex, Fft, FftPlanner};
use tokio::sync::mpsc::Receiver;
use ulid::Ulid;

use crate::{consts::*, decode::Song, plot::*};

pub struct Peak {
    pub time: usize,
    pub freq: usize,
}

pub fn spectrogram_to_sorted_peaks(spec: &[f32]) -> Vec<Peak> {
    let start = SystemTime::now();
    let peaks = get_2d_local_max(spec, OVERLAP, spec.len(), MIN_AMP);
    let end = SystemTime::now();

    if DEBUG {
        println!("Plotting spectrogram");
        plot_spectrogram(spec, OVERLAP, spec.len() / OVERLAP).expect("Failed to plot spectrogram");
        println!("Plotting peaks");
        plot_peaks(&peaks, OVERLAP, spec.len() / OVERLAP).expect("Failed to plot peaks");
    }

    println!(
        "spectrogram_to_sorted_peaks ({:?}ms)",
        end.duration_since(start).unwrap().as_millis()
    );
    peaks
}

/// Function to apply a Hamming window to a slice of data
fn apply_hamming_window(window: &mut [f32]) {
    let n = window.len();
    for (i, x) in window.iter_mut().enumerate() {
        *x *= 0.54 - 0.46 * (2.0 * std::f32::consts::PI * i as f32 / (n as f32 - 1.0)).cos();
    }
}

fn compute_window(fft: &mut Arc<dyn Fft<f32>>, window: &mut [f32]) -> Vec<f32> {
    apply_hamming_window(window);

    let mut buffer: Vec<Complex<f32>> = window.iter().map(Complex::from).collect();
    fft.process(&mut buffer);

    buffer
        .iter()
        .enumerate()
        .filter_map(|(i, c)| {
            if i < FFT_SIZE / 2 {
                Some(c.norm() / (FFT_SIZE as f32).sqrt())
            } else {
                None
            }
        })
        .collect::<Vec<f32>>()
}

/// Function to compute the spectrogram
pub async fn mp3_frames_to_spectrogram(mut rx: Receiver<Frame>) -> Song {
    let mut song = Song {
        n_channels: 2,
        channels: (None, None),
        spectrograms: (None, None),
        sample_rate: 44100,
        length_sec: 250.0,
    };

    let mut fft_1: Arc<dyn Fft<f32>> = FftPlanner::new().plan_fft_forward(FFT_SIZE);
    let mut fft_2 = FftPlanner::new().plan_fft_forward(FFT_SIZE);

    let mut channel_1_buffer: VecDeque<f32> = VecDeque::with_capacity(16 * FFT_SIZE);
    let mut channel_2_buffer: VecDeque<f32> = VecDeque::with_capacity(16 * FFT_SIZE);
    let mut spectrogram_1: Vec<f32> = vec![];
    let mut spectrogram_2: Vec<f32> = vec![];

    let mut ptr_1: usize = 0;
    let mut ptr_2: usize = 0;

    while let Some(f) = rx.recv().await {
        song.sample_rate = f.sample_rate as usize;

        channel_1_buffer.extend(f.data.iter().enumerate().filter_map(|(i, s)| {
            if i % 2 == 0 {
                Some(*s as f32 / i16::MAX as f32)
            } else {
                None
            }
        }));
        channel_2_buffer.extend(f.data.iter().enumerate().filter_map(|(i, s)| {
            if i % 2 == 1 {
                Some(*s as f32 / i16::MAX as f32)
            } else {
                None
            }
        }));

        while channel_1_buffer.len() - ptr_1 > FFT_SIZE {
            spectrogram_1.append(&mut compute_window(
                &mut fft_1,
                &mut channel_1_buffer
                    .range(ptr_1..ptr_1 + FFT_SIZE)
                    .copied()
                    .collect::<Vec<f32>>(),
            ));
            ptr_1 += OVERLAP;
        }
        while channel_2_buffer.len() - ptr_2 > FFT_SIZE {
            spectrogram_2.append(&mut compute_window(
                &mut fft_2,
                &mut channel_1_buffer
                    .range(ptr_1..ptr_1 + FFT_SIZE)
                    .copied()
                    .collect::<Vec<f32>>(),
            ));
            ptr_2 += OVERLAP;
        }
    }

    song.length_sec = channel_1_buffer.len() as f32 / song.sample_rate as f32;
    song.channels.0 = Some(channel_1_buffer.into());
    song.channels.1 = Some(channel_2_buffer.into());
    song.spectrograms.0 = Some(spectrogram_1);
    song.spectrograms.1 = Some(spectrogram_2);

    song
}

pub fn get_2d_local_max(data: &[f32], width: usize, height: usize, min: f32) -> Vec<Peak> {
    let start = SystemTime::now();
    let mask_arc_mutex = Mutex::new(vec![0_u8; data.len()]);

    (0..(data.len() / (FOOTPRINT_SIZE * FOOTPRINT_SIZE)))
        .into_par_iter()
        .for_each(|i| {
            let mut max_value: f32 = 0.0;
            let mut max_value_idx = 0;

            let start_x = i % (width / FOOTPRINT_SIZE) * FOOTPRINT_SIZE;
            let start_y = i / (width / FOOTPRINT_SIZE) * FOOTPRINT_SIZE;

            for y in start_y..std::cmp::min(start_y + FOOTPRINT_SIZE, height) {
                for x in start_x..std::cmp::min(start_x + FOOTPRINT_SIZE, width) {
                    let index = y * width + x;
                    if index >= data.len() {
                        continue;
                    }

                    if data[index] > max_value {
                        max_value = data[index];
                        max_value_idx = index;
                    }
                }
            }

            if max_value > min {
                let mut guard = mask_arc_mutex.lock().unwrap();
                guard[max_value_idx] = 1;
            }
        });

    let end = SystemTime::now();
    println!(
        "get_2d_local_max ({:?}ms)",
        end.duration_since(start).unwrap().as_millis()
    );

    let mut peaks = mask_arc_mutex
        .into_inner()
        .expect("Failed to acquire inner data from mutex")
        .par_iter()
        .enumerate()
        .filter_map(|(i, p)| {
            let x = i % width;
            let y = i / width;

            if *p == 1 {
                Some(Peak { time: y, freq: x })
            } else {
                None
            }
        })
        .collect::<Vec<Peak>>();

    peaks.sort_by(|a, b| a.time.cmp(&b.time));

    peaks
}

pub struct Fingerprint {
    pub hash: String,
    pub time: usize,
}

pub fn sorted_peaks_to_fingerprints(sorted_peaks: &[Peak]) -> Vec<Fingerprint> {
    let start = SystemTime::now();

    let ret = (0..sorted_peaks.len())
        .into_par_iter()
        .map(|i| {
            (1..FAN_VALUE)
                .map(|j| {
                    if i + j >= sorted_peaks.len() {
                        return None;
                    }

                    let f1 = sorted_peaks[i].freq;
                    let f2 = sorted_peaks[i + j].freq;
                    let t1 = sorted_peaks[i].time;
                    let t2 = sorted_peaks[i + j].time;
                    let d = t2 - t1;

                    if MIN_DELTA_TIME < d && d < MAX_DELTA_TIME {
                        return Some(Fingerprint {
                            hash: format!("{:x}", md5::compute(format!("{}|{}|{}", f1, f2, d))),
                            time: t1,
                        });
                    }

                    None
                })
                .collect::<Vec<_>>()
        })
        .flatten()
        .filter_map(|v| v)
        .collect::<Vec<_>>();

    let end = SystemTime::now();
    println!(
        "sorted_peaks_to_fingerprints ({:?}ms)",
        end.duration_since(start).unwrap().as_millis()
    );

    ret
}

pub struct ReferenceSample {
    pub id: Ulid,
    pub fingerprints: Vec<Fingerprint>,
    pub timesteps: usize,
    pub length_sec: f32,
}
