//! Client connection handling.

use super::super::protocol::{read_message, write_message, Request, Response};
use anyhow::Result;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Read the auth token from the daemon token file.
pub fn read_auth_token(work_dir: &Path) -> Option<String> {
    let token_path = work_dir.join("daemon.token");
    std::fs::read_to_string(token_path)
        .ok()
        .map(|s| s.trim().to_string())
}

/// Verify that the provided token matches the stored token.
fn verify_auth_token(work_dir: &Path, provided_token: &str) -> bool {
    if let Some(expected_token) = read_auth_token(work_dir) {
        // Constant-time comparison to prevent timing attacks
        expected_token.len() == provided_token.len()
            && expected_token
                .as_bytes()
                .iter()
                .zip(provided_token.as_bytes())
                .fold(0u8, |acc, (a, b)| acc | (a ^ b))
                == 0
    } else {
        false
    }
}

/// Handle a client connection.
pub fn handle_client_connection(
    mut stream: UnixStream,
    shutdown_flag: Arc<AtomicBool>,
    status_subscribers: Arc<Mutex<Vec<UnixStream>>>,
    log_subscribers: Arc<Mutex<Vec<UnixStream>>>,
    work_dir: &Path,
) -> Result<()> {
    // Ensure stream is in blocking mode - on macOS, accepted streams from
    // a non-blocking listener may inherit non-blocking mode, causing
    // read_message to fail with WouldBlock immediately.
    stream.set_nonblocking(false)?;

    loop {
        let request: Request = match read_message(&mut stream) {
            Ok(req) => req,
            Err(_) => {
                // Client disconnected or error reading
                break;
            }
        };

        // Extract and verify auth token from request
        let (auth_token, request_type) = match &request {
            Request::Ping { auth_token } => (auth_token, "Ping"),
            Request::Stop { auth_token } => (auth_token, "Stop"),
            Request::SubscribeStatus { auth_token } => (auth_token, "SubscribeStatus"),
            Request::SubscribeLogs { auth_token } => (auth_token, "SubscribeLogs"),
            Request::Unsubscribe { auth_token } => (auth_token, "Unsubscribe"),
        };

        if !verify_auth_token(work_dir, auth_token) {
            eprintln!("Authentication failed for {} request", request_type);
            write_message(&mut stream, &Response::AuthenticationFailed)?;
            break;
        }

        match request {
            Request::Ping { .. } => {
                write_message(&mut stream, &Response::Pong)?;
            }
            Request::Stop { .. } => {
                write_message(&mut stream, &Response::Ok)?;
                shutdown_flag.store(true, Ordering::SeqCst);
                break;
            }
            Request::SubscribeStatus { .. } => {
                if let Ok(stream_clone) = stream.try_clone() {
                    // Acquire lock, add subscriber, release lock before I/O
                    let lock_result = status_subscribers.lock().map(|mut subs| {
                        subs.push(stream_clone);
                    });
                    // Write response AFTER releasing the lock
                    if lock_result.is_ok() {
                        write_message(&mut stream, &Response::Ok)?;
                    } else {
                        write_message(
                            &mut stream,
                            &Response::Error {
                                message: "Failed to acquire subscriber lock".to_string(),
                            },
                        )?;
                    }
                } else {
                    write_message(
                        &mut stream,
                        &Response::Error {
                            message: "Failed to clone stream".to_string(),
                        },
                    )?;
                }
            }
            Request::SubscribeLogs { .. } => {
                if let Ok(stream_clone) = stream.try_clone() {
                    // Acquire lock, add subscriber, release lock before I/O
                    let lock_result = log_subscribers.lock().map(|mut subs| {
                        subs.push(stream_clone);
                    });
                    // Write response AFTER releasing the lock
                    if lock_result.is_ok() {
                        write_message(&mut stream, &Response::Ok)?;
                    } else {
                        write_message(
                            &mut stream,
                            &Response::Error {
                                message: "Failed to acquire subscriber lock".to_string(),
                            },
                        )?;
                    }
                } else {
                    write_message(
                        &mut stream,
                        &Response::Error {
                            message: "Failed to clone stream".to_string(),
                        },
                    )?;
                }
            }
            Request::Unsubscribe { .. } => {
                write_message(&mut stream, &Response::Ok)?;
                break;
            }
        }
    }

    Ok(())
}
