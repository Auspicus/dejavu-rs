use std::time::SystemTime;

use minimp3::{Decoder, Error, Frame};
use tokio::{io::AsyncRead, sync::mpsc::Sender};

pub struct Song {
    pub channels: (Option<Vec<f32>>, Option<Vec<f32>>),
    pub spectrograms: (Option<Vec<f32>>, Option<Vec<f32>>),
    pub n_channels: usize,
    pub sample_rate: usize,
    pub length_sec: f32,
}

/// Get channels from (MP3) buffer (assumes 2 channels)
pub async fn song_from_mp3_buffer(buff: &[u8]) -> Song {
    let start = SystemTime::now();
    let mut decoder = Decoder::new(std::io::Cursor::new(&buff));
    let mut sample_bytes: Vec<f32> = Vec::new();
    let mut channel_sample_rate: usize = 0;

    let start_frame_iter = SystemTime::now();
    loop {
        match decoder.next_frame_future().await {
            Ok(Frame {
                data, sample_rate, ..
            }) => {
                sample_bytes.append(
                    &mut data
                        .iter()
                        .map(|d| *d as f32 / i16::MAX as f32)
                        .collect::<Vec<f32>>(),
                );
                channel_sample_rate = sample_rate as usize;
            }
            Err(Error::Eof) => break,
            Err(e) => panic!("{:?}", e),
        }
    }
    let end_frame_iter = SystemTime::now();
    println!(
        "song_from_mp3_buffer/frame_iter ({:?}ms)",
        end_frame_iter
            .duration_since(start_frame_iter)
            .unwrap()
            .as_millis()
    );

    let channel_0: Vec<f32> = sample_bytes.iter().step_by(2).copied().collect();
    let mut channel_1_iter = sample_bytes.iter();
    channel_1_iter.next();
    let channel_1: Vec<f32> = channel_1_iter.step_by(2).copied().collect();
    let channel_0_len = &channel_0.len();

    let end = SystemTime::now();
    println!(
        "song_from_mp3_buffer ({:?}ms)",
        end.duration_since(start).unwrap().as_millis()
    );

    Song {
        channels: (Some(channel_0), Some(channel_1)),
        spectrograms: (None, None),
        n_channels: 2,
        sample_rate: channel_sample_rate,
        length_sec: *channel_0_len as f32 / channel_sample_rate as f32,
    }
}

pub async fn bytes_to_mp3_frames(rv: impl AsyncRead + Unpin, tx: Sender<Frame>) {
    let mut decoder = Decoder::new(rv);

    loop {
        match decoder.next_frame_future().await {
            Ok(frame) => {
                tx.send(frame).await.unwrap();
            }
            Err(Error::Eof) => break,
            Err(e) => panic!("{:?}", e),
        }
    }
}
