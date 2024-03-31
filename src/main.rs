use axum::{
    extract::{DefaultBodyLimit, Multipart, Path},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use dejavu_rs::align::*;
use dejavu_rs::decode::*;
use dejavu_rs::{
    consts::OVERLAP,
    fingerprint::*,
    store::{MemoryStore, Store},
};
use futures_util::TryStreamExt;
use lazy_static::lazy_static;
use minimp3::Frame;
use serde::Serialize;
use std::{num::NonZeroUsize, time::SystemTime};
use tokio::{
    io::{self},
    sync::{mpsc, Mutex},
};
use tokio_util::io::StreamReader;
use ulid::Ulid;

lazy_static! {
    static ref STORAGE: Mutex<MemoryStore> =
        Mutex::new(MemoryStore::new(NonZeroUsize::new(8).unwrap()));
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(root))
        .route("/api/reference", post(create_reference))
        .route("/api/reference/:reference_id/compare", post(compare_sample))
        .layer(DefaultBodyLimit::max(1024 * 1024 * 32));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn root() -> &'static str {
    "OK"
}

#[derive(Serialize)]
struct UploadSourceResponse {
    id: String,
}

async fn create_reference(
    mut multipart: Multipart,
) -> Result<Json<UploadSourceResponse>, (StatusCode, String)> {
    let field = multipart
        .next_field()
        .await
        .map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()))?
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                "Failed to find form field".to_string(),
            )
        })?;

    let rv = StreamReader::new(field.map_err(|e| io::Error::new(io::ErrorKind::Other, e)));

    let start = SystemTime::now();
    let (tx, rx) = mpsc::channel::<Frame>(1024);
    let (_, song) = tokio::join!(bytes_to_mp3_frames(rv, tx), mp3_frames_to_spectrogram(rx));
    println!(
        "bytes_to_mp3_frames + mp3_frames_to_spectrogram ({:?}ms)",
        SystemTime::now().duration_since(start).unwrap().as_millis()
    );

    let spectrogram = &song.spectrograms.0.unwrap();
    let peaks = spectrogram_to_sorted_peaks(spectrogram);
    let fingerprints = sorted_peaks_to_fingerprints(&peaks);
    let song_id = Ulid::new();

    let start = SystemTime::now();
    STORAGE.lock().await.set_reference_sample(
        song_id,
        ReferenceSample {
            id: song_id,
            fingerprints,
            timesteps: spectrogram.len() / OVERLAP,
            length_sec: song.length_sec,
        },
    );
    let end = SystemTime::now();
    println!(
        "set_reference_sample ({:?}ms)",
        end.duration_since(start).unwrap().as_millis()
    );

    Ok(Json(UploadSourceResponse { id: song_id.into() }))
}

#[derive(Serialize)]
struct UploadSampleResponse {
    offset_seconds: f32,
    sample_first_match_seconds: f32,
}

async fn compare_sample(
    Path(reference_id): Path<String>,
    mut multipart: Multipart,
) -> Result<Json<UploadSampleResponse>, (StatusCode, String)> {
    let ulid = Ulid::from_string(&reference_id)
        .map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()))?;

    let field = multipart
        .next_field()
        .await
        .map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()))?
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                "Failed to find form field".to_string(),
            )
        })?;

    let rv = StreamReader::new(field.map_err(|e| io::Error::new(io::ErrorKind::Other, e)));

    let start = SystemTime::now();
    let (tx, rx) = mpsc::channel::<Frame>(1024);
    let (_, song) = tokio::join!(bytes_to_mp3_frames(rv, tx), mp3_frames_to_spectrogram(rx));
    println!(
        "bytes_to_mp3_frames + mp3_frames_to_spectrogram ({:?}ms)",
        SystemTime::now().duration_since(start).unwrap().as_millis()
    );

    let spectrogram = &song.spectrograms.0.unwrap();
    let peaks = spectrogram_to_sorted_peaks(spectrogram);
    let sample_fingerprints = sorted_peaks_to_fingerprints(&peaks);

    let mut guard = STORAGE.lock().await;
    let matching_file = guard
        .get_reference_sample(&ulid)
        .expect("Failed to lookup source");
    let sample_offset = align_fingerprints(&matching_file.fingerprints, &sample_fingerprints)
        .expect("Failed to find offset");

    Ok(Json(UploadSampleResponse {
        offset_seconds: matching_file.length_sec
            * (sample_offset.most_common_offset as f32 / matching_file.timesteps as f32),
        sample_first_match_seconds: song.length_sec
            * (sample_offset.first_sample_offset_match as f32
                / (spectrogram.len() as f32 / OVERLAP as f32)),
    }))
}
