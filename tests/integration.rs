use vsp_router::{create_virtual_serial_port, transfer};

use bytes::{Bytes, BytesMut};
use futures_util::future::{AbortHandle, Abortable};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::timeout;
use tokio_serial::SerialPortBuilderExt;
use tokio_stream::StreamMap;
use tokio_util::io::ReaderStream;

use std::collections::HashMap;
use std::time::Duration;

#[tokio::test]
async fn virtual_routes() {
    // let _ = tracing_subscriber::fmt::try_init();

    let baud_rate = 115200;

    let (sources, (sinks, _symlinks)): (Vec<_>, (Vec<_>, Vec<_>)) = (0..3)
        .map(|id| {
            let (port, symlink) = create_virtual_serial_port(id.to_string()).unwrap();
            let (reader, writer) = tokio::io::split(port);
            let reader_stream = ReaderStream::new(reader);
            (reader_stream, (writer, symlink))
        })
        .unzip();
    let sources: StreamMap<String, ReaderStream<_>> = sources
        .into_iter()
        .enumerate()
        .map(|(id, source)| (id.to_string(), source))
        .collect();
    let sinks: HashMap<String, _> = sinks
        .into_iter()
        .enumerate()
        .map(|(id, sink)| (id.to_string(), sink))
        .collect();

    let mut end_points = (0..3)
        .map(|id| {
            tokio_serial::new(id.to_string(), baud_rate)
                .open_native_async()
                .unwrap()
        })
        .collect::<Vec<_>>();

    let routes: HashMap<String, Vec<String>> = [
        ("0".to_owned(), vec!["2".to_owned()]),
        ("1".to_owned(), vec!["2".to_owned()]),
        ("2".to_owned(), vec!["0".to_owned(), "1".to_owned()]),
    ]
    .into_iter()
    .collect();

    let (abort_handle, abort_registration) = AbortHandle::new_pair();

    let join_handle = tokio::spawn(async move {
        Abortable::new(transfer(sources, sinks, routes), abort_registration)
            .await
            .map(|transfer_result| transfer_result.unwrap())
            .ok()
    });

    timeout(Duration::from_secs(1), async move {
        for _ in 0..2 {
            let msg = Bytes::from("from 0");
            end_points[0].write_all(&msg).await.unwrap();
            let mut buf = BytesMut::new();
            end_points[2].read_buf(&mut buf).await.unwrap();
            assert_eq!(msg, buf);

            let msg = Bytes::from("from 1");
            end_points[1].write_all(&msg).await.unwrap();
            let mut buf = BytesMut::new();
            end_points[2].read_buf(&mut buf).await.unwrap();
            assert_eq!(msg, buf);

            let msg = Bytes::from("from 2");
            end_points[2].write_all(&msg).await.unwrap();
            let mut buf = BytesMut::new();
            end_points[0].read_buf(&mut buf).await.unwrap();
            assert_eq!(msg, buf);
            let mut buf = BytesMut::new();
            end_points[1].read_buf(&mut buf).await.unwrap();
            assert_eq!(msg, buf);
        }
    })
    .await
    .expect("test took longer than expected");

    abort_handle.abort();
    join_handle.await.unwrap();
}
