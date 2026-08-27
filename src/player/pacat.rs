//! Local audio through PulseAudio's `pacat`, for machines where ALSA has no
//! device but a Pulse server is reachable - a stock WSL distribution.
//!
//! librespot ships a generic subprocess sink, but it cannot tell a working
//! helper from a dead one: it treats any exit as success, respawns on every
//! failed write (a fresh pipe buffer swallows the next 64 KB, so the write
//! "succeeds"), and lets the child inherit stdout/stderr - pacat's error text
//! would land on the TUI. This sink spawns `pacat` with its output discarded,
//! reports a dead child as a write error so the player pauses and the UI can
//! say so, and offers a real probe that feeds silence and checks the process
//! survived it.

use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use librespot::playback::audio_backend::{Sink, SinkError, SinkResult};
use librespot::playback::config::AudioFormat;
use librespot::playback::convert::Converter;
use librespot::playback::decoder::AudioPacket;

/// librespot decodes to 44.1 kHz stereo.
pub const SAMPLE_RATE: u32 = 44_100;
pub const CHANNELS: u32 = 2;
/// The one sample format this sink emits; `f64` samples are converted to it.
pub const FORMAT: AudioFormat = AudioFormat::S16;

/// pacat's name for `format`, when it has one.
pub fn pacat_sample_format(format: AudioFormat) -> Option<&'static str> {
    match format {
        AudioFormat::S16 => Some("s16le"),
        AudioFormat::S32 => Some("s32le"),
        AudioFormat::S24 => Some("s24-32le"),
        AudioFormat::S24_3 => Some("s24le"),
        AudioFormat::F32 => Some("float32le"),
        AudioFormat::F64 => None,
    }
}

/// The `pacat` invocation matching what this sink writes.
pub fn pacat_command() -> String {
    let format = pacat_sample_format(FORMAT).expect("FORMAT is one pacat understands");
    format!(
        "pacat --playback --raw --format={format} --rate={SAMPLE_RATE} --channels={CHANNELS} --client-name=joshify --stream-name=Spotify"
    )
}

/// `pacat` if it is on `PATH`.
pub fn pacat_on_path() -> Option<String> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join("pacat"))
        .any(|candidate| candidate.is_file())
        .then(pacat_command)
}

/// Audio sink that pipes PCM into a helper process.
pub struct PacatSink {
    program: String,
    args: Vec<String>,
    child: Option<Child>,
}

impl PacatSink {
    /// `command` is split on whitespace; there is no shell.
    pub fn new(command: &str) -> Self {
        let mut words = command.split_whitespace().map(str::to_string);
        Self {
            program: words.next().unwrap_or_default(),
            args: words.collect(),
            child: None,
        }
    }

    fn write_bytes(&mut self, data: &[u8]) -> SinkResult<()> {
        let child = self
            .child
            .as_mut()
            .ok_or_else(|| SinkError::NotConnected(format!("{} is not running", self.program)))?;
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| SinkError::NotConnected(format!("{} has no stdin", self.program)))?;
        if let Err(e) = stdin.write_all(data) {
            // The helper is gone. Do not respawn: a fresh one would accept a
            // pipe buffer's worth and die the same way, and the user would hear
            // silence while the position advances. Report it so the player
            // pauses and the UI can say so.
            let exit = match child.try_wait() {
                Ok(Some(status)) => status.to_string(),
                _ => "still running".to_string(),
            };
            let _ = child.kill();
            let _ = child.wait();
            self.child = None;
            return Err(SinkError::OnWrite(format!(
                "{} stopped accepting audio ({exit}): {e}",
                self.program
            )));
        }
        Ok(())
    }

    /// Whether the helper has exited; `Some(status)` if so.
    fn exited(&mut self) -> Result<Option<String>, String> {
        match self.child.as_mut() {
            None => Ok(Some("never started".to_string())),
            Some(child) => match child.try_wait() {
                Ok(Some(status)) => Ok(Some(status.to_string())),
                Ok(None) => Ok(None),
                Err(e) => Err(format!("could not poll {}: {e}", self.program)),
            },
        }
    }
}

impl Sink for PacatSink {
    fn start(&mut self) -> SinkResult<()> {
        if self.child.is_none() {
            let child = Command::new(&self.program)
                .args(&self.args)
                .stdin(Stdio::piped())
                // pacat's diagnostics must not reach the terminal the TUI owns.
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| {
                    SinkError::ConnectionRefused(format!("could not start {}: {e}", self.program))
                })?;
            self.child = Some(child);
        }
        Ok(())
    }

    fn stop(&mut self) -> SinkResult<()> {
        if let Some(mut child) = self.child.take() {
            drop(child.stdin.take());
            let _ = child.kill();
            let _ = child.wait();
        }
        Ok(())
    }

    fn write(&mut self, packet: AudioPacket, converter: &mut Converter) -> SinkResult<()> {
        let bytes: Vec<u8> = match packet {
            AudioPacket::Samples(samples) => converter
                .f64_to_s16(&samples)
                .iter()
                .flat_map(|sample| sample.to_le_bytes())
                .collect(),
            AudioPacket::Raw(raw) => raw,
        };
        self.write_bytes(&bytes)
    }
}

/// Prove `command` can take audio: start it, feed it 200 ms of silence, and
/// check it is still alive `deadline` later.
///
/// Spawning alone proves nothing - pacat connects and creates its stream on
/// the first write, and without a server it exits within milliseconds of
/// that. A helper that hangs connecting is bounded by the deadline and killed.
/// 200 ms of S16 stereo is ~35 KB, under the pipe buffer, so the write never
/// blocks even if the helper reads nothing.
pub fn probe(command: &str, deadline: Duration) -> Result<(), String> {
    let mut sink = PacatSink::new(command);
    sink.start().map_err(|e| e.to_string())?;
    let silence = vec![0u8; (SAMPLE_RATE * CHANNELS * 2 / 5) as usize];
    let fed = sink.write_bytes(&silence).map_err(|e| e.to_string());
    let outcome = fed.and_then(|()| {
        let started = Instant::now();
        while started.elapsed() < deadline {
            if let Some(status) = sink.exited()? {
                return Err(format!(
                    "{} exited during the probe ({status})",
                    sink.program
                ));
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        Ok(())
    });
    let _ = sink.stop();
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEADLINE: Duration = Duration::from_millis(300);

    /// `cat` stands in for a healthy pacat: it takes PCM on stdin and stays up.
    #[test]
    fn a_helper_that_keeps_reading_passes_the_probe() {
        assert_eq!(probe("cat", DEADLINE), Ok(()));
    }

    /// `false` stands in for pacat with no server: it exits at once. This is
    /// the case librespot's own subprocess sink reports as success.
    #[test]
    fn a_helper_that_exits_at_once_fails_the_probe() {
        let err = probe("false", DEADLINE).expect_err("an exited helper is not an output");
        assert!(err.contains("exited"), "{err}");
    }

    #[test]
    fn a_helper_that_cannot_be_spawned_fails_the_probe() {
        let err = probe("/nonexistent/joshify-fake-pacat", DEADLINE).unwrap_err();
        assert!(err.contains("could not start"), "{err}");
    }

    /// Once the helper dies, writes fail instead of quietly restarting it.
    #[test]
    fn writes_after_the_helper_dies_are_errors_not_respawns() {
        let mut sink = PacatSink::new("false");
        sink.start().unwrap();
        std::thread::sleep(Duration::from_millis(100));
        let big = vec![0u8; 1 << 20]; // more than any pipe buffer
        let err = sink.write_bytes(&big).expect_err("the pipe is closed");
        assert!(matches!(err, SinkError::OnWrite(_)), "{err}");
        assert!(sink.child.is_none(), "no child must be kept or respawned");
        let again = sink.write_bytes(&[0; 4]).expect_err("still not connected");
        assert!(matches!(again, SinkError::NotConnected(_)), "{again}");
    }

    #[test]
    fn samples_are_converted_to_little_endian_s16() {
        let mut sink = PacatSink::new("cat");
        sink.start().unwrap();
        let mut converter = Converter::new(None);
        sink.write(AudioPacket::Samples(vec![0.0; 4]), &mut converter)
            .expect("cat accepts anything");
        sink.stop().unwrap();
    }

    #[test]
    fn the_command_line_matches_the_sink_format() {
        let cmd = pacat_command();
        assert!(cmd.starts_with("pacat "));
        assert!(cmd.contains("--format=s16le"), "{cmd}");
        assert!(cmd.contains("--rate=44100"), "{cmd}");
        assert!(cmd.contains("--channels=2"), "{cmd}");
        assert_eq!(pacat_sample_format(AudioFormat::F64), None);
    }
}
