//! Complete, bounded request reads and owned server tasks for synthetic HTTP tests.
use std::{
    future::Future,
    io,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    task::{JoinError, JoinHandle},
};

const REQUEST_LIMIT: usize = 16_384;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

pub(crate) async fn read_request(reader: &mut (impl AsyncRead + Unpin)) -> io::Result<Vec<u8>> {
    tokio::time::timeout(REQUEST_TIMEOUT, async {
        let mut request = Vec::new();
        let mut expected = None;
        loop {
            let mut chunk = [0; 2048];
            let count = reader.read(&mut chunk).await?;
            if count == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "incomplete test request",
                ));
            }
            if count > REQUEST_LIMIT.saturating_sub(request.len()) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "test request too large",
                ));
            }
            request.extend_from_slice(&chunk[..count]);
            if expected.is_none()
                && let Some(end) = request.windows(4).position(|w| w == b"\r\n\r\n")
            {
                let headers = std::str::from_utf8(&request[..end])
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                let mut length = None;
                for line in headers.lines().skip(1) {
                    let (name, value) = line.split_once(':').ok_or_else(|| {
                        io::Error::new(io::ErrorKind::InvalidData, "invalid test request header")
                    })?;
                    if name.eq_ignore_ascii_case("transfer-encoding") {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "test reader requires a fixed-length request",
                        ));
                    }
                    if name.eq_ignore_ascii_case("content-length") {
                        let value = value
                            .trim()
                            .parse::<usize>()
                            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                        if length.replace(value).is_some() {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "duplicate content length",
                            ));
                        }
                    }
                }
                let length = length.unwrap_or(0);
                if length > REQUEST_LIMIT.saturating_sub(end + 4) {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "test request too large",
                    ));
                }
                expected = Some(end + 4 + length);
            }
            if let Some(expected) = expected
                && request.len() >= expected
            {
                request.truncate(expected);
                return Ok(request);
            }
        }
    })
    .await
    .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "test request stalled"))?
}

// Unlike a bare JoinHandle, dropping this handle aborts the server on assertion
// failure. The timeout bounds accept/write waits and missing-request failures.
pub(crate) struct ServerTask<T> {
    task: JoinHandle<T>,
}
pub(crate) fn spawn_server<T: Send + 'static>(
    future: impl Future<Output = T> + Send + 'static,
) -> ServerTask<T> {
    ServerTask {
        task: tokio::spawn(async {
            tokio::time::timeout(Duration::from_secs(15), future)
                .await
                .expect("synthetic HTTP server timed out")
        }),
    }
}
impl<T> ServerTask<T> {
    pub(crate) fn abort(&self) {
        self.task.abort();
    }
}
impl<T> Drop for ServerTask<T> {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl<T> Future for ServerTask<T> {
    type Output = Result<T, JoinError>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.task).poll(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn fragmented_headers_and_body_are_captured_completely() {
        let (mut writer, mut reader) = tokio::io::duplex(8);
        let task = spawn_server(async move {
            for part in [
                "POST / HTTP/1.1\r\nCont",
                "ent-Length: 5\r\nX-Test: complete\r\n\r\n",
                "he",
                "llo",
            ] {
                writer.write_all(part.as_bytes()).await.unwrap();
                tokio::task::yield_now().await;
            }
        });
        assert_eq!(
            read_request(&mut reader).await.unwrap(),
            b"POST / HTTP/1.1\r\nContent-Length: 5\r\nX-Test: complete\r\n\r\nhello"
        );
        task.await.unwrap();
    }

    #[tokio::test(start_paused = true)]
    async fn oversized_truncated_and_stalled_requests_fail_boundedly() {
        for bytes in [
            &b"POST / HTTP/1.1\r\nContent-Length: 999999\r\n\r\n"[..],
            &b"POST / HTTP/1.1\r\nContent-Length: 5\r\n\r\nhi"[..],
        ] {
            assert!(read_request(&mut &bytes[..]).await.is_err());
        }
        let (_writer, mut reader) = tokio::io::duplex(8);
        assert_eq!(
            read_request(&mut reader).await.unwrap_err().kind(),
            io::ErrorKind::TimedOut
        );
    }

    #[tokio::test]
    async fn dropping_server_task_cancels_its_work() {
        let (started, ready) = tokio::sync::oneshot::channel();
        let (sender, closed) = tokio::sync::oneshot::channel::<()>();
        let task = spawn_server(async move {
            let _keep_until_cancelled = sender;
            started.send(()).unwrap();
            std::future::pending::<()>().await;
        });
        ready.await.unwrap();
        drop(task);
        assert!(
            tokio::time::timeout(Duration::from_secs(1), closed)
                .await
                .unwrap()
                .is_err()
        );
    }
}
