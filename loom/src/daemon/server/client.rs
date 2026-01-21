//! Client connection handling.

use super::super::protocol::{read_message, write_message, Request, Response};
use anyhow::Result;
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Handle a client connection.
pub fn handle_client_connection(
    mut stream: UnixStream,
    shutdown_flag: Arc<AtomicBool>,
    status_subscribers: Arc<Mutex<Vec<UnixStream>>>,
    log_subscribers: Arc<Mutex<Vec<UnixStream>>>,
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

        match request {
            Request::Ping => {
                write_message(&mut stream, &Response::Pong)?;
            }
            Request::Stop => {
                write_message(&mut stream, &Response::Ok)?;
                shutdown_flag.store(true, Ordering::Relaxed);
                break;
            }
            Request::SubscribeStatus => {
                if let Ok(stream_clone) = stream.try_clone() {
                    match status_subscribers.lock() {
                        Ok(mut subs) => {
                            subs.push(stream_clone);
                            write_message(&mut stream, &Response::Ok)?;
                        }
                        Err(_) => {
                            write_message(
                                &mut stream,
                                &Response::Error {
                                    message: "Failed to acquire subscriber lock".to_string(),
                                },
                            )?;
                        }
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
            Request::SubscribeLogs => {
                if let Ok(stream_clone) = stream.try_clone() {
                    match log_subscribers.lock() {
                        Ok(mut subs) => {
                            subs.push(stream_clone);
                            write_message(&mut stream, &Response::Ok)?;
                        }
                        Err(_) => {
                            write_message(
                                &mut stream,
                                &Response::Error {
                                    message: "Failed to acquire subscriber lock".to_string(),
                                },
                            )?;
                        }
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
            Request::Unsubscribe => {
                write_message(&mut stream, &Response::Ok)?;
                break;
            }
        }
    }

    Ok(())
}
