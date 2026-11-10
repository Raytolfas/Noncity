use crate::cipher::CryptState;
use pin_project_lite::pin_project;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

pin_project! {
    pub struct CipherStream<S> {
        #[pin]
        inner: S,
        crypt: Option<CryptState>,
    }
}

impl<S> CipherStream<S> {
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            crypt: None,
        }
    }

    pub fn enable_crypto(&mut self, key: &[u8; 16]) {
        self.crypt = Some(CryptState::new(key));
    }
}

impl<S: AsyncRead> AsyncRead for CipherStream<S> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.project();
        let prev_len = buf.filled().len();
        let poll = this.inner.poll_read(cx, buf);

        if let Poll::Ready(Ok(())) = poll {
            let new_len = buf.filled().len();
            if new_len > prev_len {
                if let Some(crypt) = this.crypt {
                    let filled = buf.filled_mut();
                    crypt.decrypt(&mut filled[prev_len..new_len]);
                }
            }
        }

        poll
    }
}

impl<S: AsyncWrite> AsyncWrite for CipherStream<S> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.project();
        if let Some(crypt) = this.crypt {
            let mut encrypted = buf.to_vec();
            crypt.encrypt(&mut encrypted);
            this.inner.poll_write(cx, &encrypted)
        } else {
            this.inner.poll_write(cx, buf)
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.project().inner.poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.project().inner.poll_shutdown(cx)
    }
}
