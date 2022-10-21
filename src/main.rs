use futures::{future, FutureExt, TryFutureExt};
use futures_util::io::AsyncReadExt;
use std::{
    io::Write,
    pin::Pin,
    task::{ready, Context, Poll},
    time::Instant,
};
use tokio::{fs::File, io::AsyncRead};
use tokio_util::compat::TokioAsyncReadCompatExt;

/// read files asynchronously in sequence
pub struct FilesReader<R> {
    files: Vec<R>,
    index: usize,
}

impl<R> FilesReader<R> {
    pub fn new(files: Vec<R>) -> Self {
        FilesReader { files, index: 0 }
    }
}

impl<R> AsyncRead for FilesReader<R>
where
    R: AsyncRead + Unpin,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let me = self.get_mut();

        while me.index < me.files.len() {
            let rem = buf.remaining();
            ready!(Pin::new(&mut me.files[me.index]).poll_read(cx, buf))?;
            if buf.remaining() == rem {
                me.index += 1;
            } else {
                return Poll::Ready(Ok(()));
            }
        }

        Poll::Ready(Ok(()))
    }
}

fn cat<R>(r: Vec<R>) -> FilesReader<R> {
    FilesReader::new(r)
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let files = future::try_join_all(std::env::args().skip(1).map(|f| File::open(f))).await?;
    let total_len = future::try_join_all(files.iter().map(|f| f.metadata()))
        .map_ok(|x| x.iter().map(|m| m.len()).sum::<u64>() as usize)
        .await?;
    let mut data = Vec::with_capacity(total_len);
    let begin = Instant::now();
    let file_len = files.len();
    cat(files).compat().read_to_end(&mut data).await?;
    eprintln!(
        "{} file(s) with total length {} (expected by meta {}) read, take {:?}",
        file_len,
        data.len(),
        total_len,
        begin.elapsed(),
    );
    let mut out = std::io::stdout().lock();
    out.write_all(&data)?;
    Ok(())
}
