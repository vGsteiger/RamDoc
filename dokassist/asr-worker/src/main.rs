//! `dokassist-asr` — RamDoc's out-of-process speech-to-text worker.
//!
//! Protocol (one job per process):
//!
//! ```text
//! dokassist-asr --model <path.gguf> [--language <iso>]
//!
//! stdout  {"event":"ready", ...}            once the model is loaded
//! stdin   raw little-endian f32 PCM, 16 kHz mono, in [-1, 1], until EOF
//! stdout  {"event":"progress","done":n,"total":m}   per transcribed chunk
//! stdout  {"event":"result","text":"...","language":...}
//!   or    {"event":"error","message":"..."}   (exit code 1)
//! ```
//!
//! Audio is streamed through the pipe and never written to disk. Native
//! logging is disabled so transcript fragments cannot leak to stderr.

mod chunking;

use serde::Serialize;
use std::io::{self, Read, Write};
use std::process::ExitCode;
use transcribe_cpp::{Model, RunOptions};

pub const SAMPLE_RATE: usize = 16_000;

/// Upper bound on accepted audio: 3 hours of 16 kHz mono f32 (~690 MB).
const MAX_SAMPLES: usize = 3 * 60 * 60 * SAMPLE_RATE;

/// Chunk length used when the model reports no practical limit. Bounding the
/// window keeps peak memory flat for long recorded dictations and lets the
/// app report progress.
const DEFAULT_CHUNK_MS: i64 = 120_000;

#[derive(Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum Event<'a> {
    Ready {
        arch: String,
        backend: String,
        languages: Vec<String>,
    },
    Progress {
        done: usize,
        total: usize,
    },
    Result {
        text: String,
        language: Option<String>,
        audio_ms: u64,
    },
    Error {
        message: &'a str,
    },
}

fn emit(event: &Event<'_>) {
    let mut out = io::stdout().lock();
    // A closed stdout means the parent went away; nothing useful to do.
    if serde_json::to_writer(&mut out, event).is_ok() {
        let _ = out.write_all(b"\n");
        let _ = out.flush();
    }
}

struct Args {
    model: String,
    language: Option<String>,
}

fn parse_args() -> Result<Args, String> {
    let mut model = None;
    let mut language = None;
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--model" => model = it.next(),
            "--language" => language = it.next().filter(|l| !l.is_empty() && l != "auto"),
            "--version" => {
                println!("dokassist-asr {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok(Args {
        model: model.ok_or("missing --model <path>")?,
        language,
    })
}

fn read_pcm() -> Result<Vec<f32>, String> {
    let mut bytes = Vec::new();
    io::stdin()
        .lock()
        .take((MAX_SAMPLES * 4 + 4) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("failed to read audio from stdin: {e}"))?;
    if bytes.len() > MAX_SAMPLES * 4 {
        return Err("audio exceeds the 3 hour limit".into());
    }
    if bytes.len() % 4 != 0 {
        return Err("audio stream is not a whole number of f32 samples".into());
    }
    Ok(bytes
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect())
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    transcribe_cpp::disable_logging();

    let model = Model::load(&args.model).map_err(|e| format!("failed to load model: {e}"))?;
    let caps = model
        .capabilities()
        .map_err(|e| format!("failed to read model capabilities: {e}"))?;
    let mut session = model
        .session()
        .map_err(|e| format!("failed to create session: {e}"))?;
    let limits = session
        .limits()
        .map_err(|e| format!("failed to read session limits: {e}"))?;

    emit(&Event::Ready {
        arch: model.arch(),
        backend: model.backend(),
        languages: caps.languages.clone(),
    });

    let pcm = read_pcm()?;
    let audio_ms = (pcm.len() as u64 * 1000) / SAMPLE_RATE as u64;
    if chunking::is_silent(&pcm) {
        emit(&Event::Result {
            text: String::new(),
            language: None,
            audio_ms,
        });
        return Ok(());
    }

    // Only pass a language hint the model actually understands; otherwise let
    // it auto-detect (Parakeet v3 is language-agnostic at the API level).
    let language = args
        .language
        .filter(|l| caps.languages.iter().any(|c| c.eq_ignore_ascii_case(l)));
    let options = RunOptions {
        language,
        ..Default::default()
    };

    let limit_ms = match limits.effective_max_audio_ms {
        ms if ms > 0 => ms.min(DEFAULT_CHUNK_MS),
        _ => DEFAULT_CHUNK_MS,
    };
    let chunks = chunking::split(&pcm, (limit_ms as usize * SAMPLE_RATE) / 1000);

    let mut pieces = Vec::with_capacity(chunks.len());
    let mut detected = None;
    for (i, range) in chunks.iter().enumerate() {
        let slice = &pcm[range.clone()];
        if !chunking::is_silent(slice) {
            let result = session
                .run(slice, &options)
                .map_err(|e| format!("transcription failed: {e}"))?;
            let text = result.text.trim().to_string();
            if !text.is_empty() {
                pieces.push(text);
            }
            if detected.is_none() {
                detected = result.language;
            }
        }
        emit(&Event::Progress {
            done: i + 1,
            total: chunks.len(),
        });
    }

    emit(&Event::Result {
        text: pieces.join(" "),
        language: detected,
        audio_ms,
    });
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            emit(&Event::Error { message: &message });
            ExitCode::FAILURE
        }
    }
}
