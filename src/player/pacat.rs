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

use std::io::{ErrorKind, Write};
use std::os::fd::AsRawFd;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use librespot::playback::audio_backend::{Sink, SinkError, SinkResult};
use librespot::playback::config::AudioFormat;
use librespot::playback::convert::Converter;
use librespot::playback::decoder::AudioPacket;
use librespot::playback::{NUM_CHANNELS, SAMPLE_RATE};

/// The one sample format this sink emits; `f64` samples are converted to it.
pub const FORMAT: AudioFormat = AudioFormat::S16;

/// How long a write may sit on a full pipe before the helper is declared
/// stuck. pacat that connected but never gets a stream reads ~4 KB and stops;
/// without this the player thread would block in `write_all` forever, and with
/// it pause, stop and quit.
const WRITE_STALL: Duration = Duration::from_secs(5);

/// How long `stop` waits for the helper to drain and exit after EOF before
/// killing it. Every other librespot backend drains on stop; killing at once
/// drops the pipe plus pacat's server-side buffer, which truncates the end of a
/// track and jumps forward on resume.
const DRAIN_DEADLINE: Duration = Duration::from_secs(3);

/// Playback latency asked of PulseAudio. Small enough that pause responds
/// promptly and `stop` drains quickly; large enough not to underrun through a
/// pipe.
const LATENCY_MS: u32 = 250;

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
        "pacat --playback --raw --format={format} --rate={SAMPLE_RATE} --channels={NUM_CHANNELS} --latency-msec={LATENCY_MS} --client-name=joshify --stream-name=Spotify"
    )
}

/// What the helper's exit looks like right now, if it has exited.
fn exit_status(child: &mut Child) -> Option<String> {
    match child.try_wait() {
        Ok(Some(status)) => Some(status.to_string()),
        Ok(None) => None,
        Err(e) => Some(format!("unknown, could not poll: {e}")),
    }
}

/// Write all of `data` to a non-blocking pipe, giving up if it makes no
/// progress for `stall`.
fn write_all_bounded(
    stdin: &mut ChildStdin,
    mut data: &[u8],
    stall: Duration,
) -> std::io::Result<()> {
    let mut last_progress = Instant::now();
    while !data.is_empty() {
        match stdin.write(data) {
            Ok(0) => {
                return Err(std::io::Error::new(
                    ErrorKind::WriteZero,
                    "pipe accepted nothing",
                ))
            }
            Ok(n) => {
                data = &data[n..];
                last_progress = Instant::now();
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                if last_progress.elapsed() >= stall {
                    return Err(std::io::Error::new(
                        ErrorKind::TimedOut,
                        format!("no audio consumed for {}s", stall.as_secs()),
                    ));
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(e) if e.kind() == ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

/// Put the helper's stdin pipe in non-blocking mode so writes can be bounded.
fn set_nonblocking(stdin: &ChildStdin) -> std::io::Result<()> {
    let fd = stdin.as_raw_fd();
    // SAFETY: fcntl on a file descriptor we own; F_GETFL/F_SETFL take no
    // pointers and cannot invalidate memory.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 {
        return Err(std::io::Error::last_os_error());
    }
    if unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
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
    /// Where write failures are reported so the UI can show them. librespot
    /// only logs a sink error and pauses; nothing else would tell the user why
    /// their music stopped.
    errors: Option<Sender<String>>,
}

impl PacatSink {
    /// `command` is split on whitespace; there is no shell.
    pub fn new(command: &str) -> Self {
        let mut words = command.split_whitespace().map(str::to_string);
        Self {
            program: words.next().unwrap_or_default(),
            args: words.collect(),
            child: None,
            errors: None,
        }
    }

    /// Report write failures on `errors` as well as returning them.
    pub fn with_error_reporting(mut self, errors: Sender<String>) -> Self {
        self.errors = Some(errors);
        self
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
        if let Err(e) = write_all_bounded(stdin, data, WRITE_STALL) {
            // The helper is gone or stuck. Do not respawn here: a fresh one
            // would accept a pipe buffer's worth and die the same way, and the
            // user would hear silence while the position advances. Return the
            // error so the player pauses, and report it so the UI can say why.
            let exit = exit_status(child).unwrap_or_else(|| "still running".to_string());
            let _ = child.kill();
            let _ = child.wait();
            self.child = None;
            let message = format!("{} stopped accepting audio ({exit}): {e}", self.program);
            if let Some(errors) = &self.errors {
                let _ = errors.send(message.clone());
            }
            return Err(SinkError::OnWrite(message));
        }
        Ok(())
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
            if let Some(stdin) = child.stdin.as_ref() {
                set_nonblocking(stdin).map_err(|e| {
                    SinkError::ConnectionRefused(format!(
                        "could not configure {}'s pipe: {e}",
                        self.program
                    ))
                })?;
            }
            self.child = Some(child);
        }
        Ok(())
    }

    fn stop(&mut self) -> SinkResult<()> {
        if let Some(mut child) = self.child.take() {
            // EOF lets pacat play out what it has buffered and exit on its
            // own; kill only a helper that does not.
            drop(child.stdin.take());
            let started = Instant::now();
            while exit_status(&mut child).is_none() {
                if started.elapsed() >= DRAIN_DEADLINE {
                    let _ = child.kill();
                    let _ = child.wait();
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
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
    let silence = vec![0u8; SAMPLE_RATE as usize * NUM_CHANNELS as usize * FORMAT.size() / 5];
    // A helper that dies before or during the write fails here; one that dies
    // shortly after fails below. Both are "exited" to the caller.
    let outcome = match sink.write_bytes(&silence) {
        Err(e) => Err(format!("{} exited during the probe: {e}", sink.program)),
        Ok(()) => {
            let started = Instant::now();
            let mut outcome = Ok(());
            while started.elapsed() < deadline {
                if let Some(status) = sink.child.as_mut().and_then(exit_status) {
                    outcome = Err(format!(
                        "{} exited during the probe ({status})",
                        sink.program
                    ));
                    break;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            outcome
        }
    };
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
        // Whether the exit is seen by the write or by the poll afterwards is a
        // scheduling race; both must read as "exited".
        let err = probe("false", DEADLINE).expect_err("an exited helper is not an output");
        assert!(err.contains("exited during the probe"), "{err}");
    }

    /// A helper that accepts the connection but never reads (pacat whose
    /// stream never becomes ready) must not wedge the player thread forever.
    #[test]
    fn a_helper_that_stops_reading_is_a_bounded_error() {
        let mut sink = PacatSink::new("sleep 30");
        sink.start().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        sink.errors = Some(tx);
        let started = Instant::now();
        let big = vec![0u8; 4 << 20]; // far more than any pipe buffer
        let err = sink.write_bytes(&big).expect_err("nothing reads the pipe");
        assert!(
            started.elapsed() < WRITE_STALL + Duration::from_secs(2),
            "write must give up"
        );
        assert!(matches!(err, SinkError::OnWrite(_)), "{err}");
        assert!(sink.child.is_none(), "the stuck helper must be gone");
        let reported = rx
            .try_recv()
            .expect("the failure must be reported for the UI");
        assert!(reported.contains("no audio consumed"), "{reported}");
    }

    /// `stop` lets the helper drain: with `cat` (exits on EOF) it returns
    /// promptly without needing to kill; with a helper ignoring EOF it kills
    /// after the deadline rather than hanging.
    #[test]
    fn stop_waits_for_the_helper_to_drain_then_kills_it() {
        let mut sink = PacatSink::new("cat");
        sink.start().unwrap();
        sink.write_bytes(&[0; 64]).unwrap();
        let started = Instant::now();
        sink.stop().unwrap();
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "cat exits on EOF at once"
        );

        let mut stubborn = PacatSink::new("sleep 30");
        stubborn.start().unwrap();
        let started = Instant::now();
        stubborn.stop().unwrap();
        let took = started.elapsed();
        assert!(
            took >= DRAIN_DEADLINE,
            "must give the helper the drain window: {took:?}"
        );
        assert!(
            took < DRAIN_DEADLINE + Duration::from_secs(2),
            "then kill it: {took:?}"
        );
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
        assert!(cmd.contains("--latency-msec="), "{cmd}");
        assert_eq!(pacat_sample_format(AudioFormat::F64), None);
    }
}
