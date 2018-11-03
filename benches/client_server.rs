// `cargo test --benches` and `#[feature(test)]` work only in nightly
#![cfg(rustc_nightly)]
#![feature(test)]

use std::sync::Arc;

extern crate bytes;
extern crate futures;
extern crate httpbis;
extern crate test;

use httpbis::*;

use futures::future::Future;
use futures::stream;
use futures::stream::Stream;

use bytes::Bytes;

use test::Bencher;

#[bench]
fn download_megabyte_in_kb_chunks(b: &mut Bencher) {
    struct Megabyte;

    impl ServerHandler for Megabyte {
        fn start_request(
            &self,
            _context: ServerHandlerContext,
            _req: ServerRequest,
            mut resp: ServerResponse,
        ) -> httpbis::Result<()> {
            resp.send_headers(Headers::ok_200())?;
            resp.pull_bytes_from_stream(stream::iter_ok(
                (0..1024).map(|i| Bytes::from(vec![(i % 0xff) as u8; 1024])),
            ))?;
            Ok(())
        }
    }

    let mut server = ServerBuilder::new_plain();
    server.set_port(0);
    server.service.set_service("/", Arc::new(Megabyte));
    let server = server.build().expect("server");

    let client = Client::new_plain(
        "127.0.0.1",
        server.local_addr().port().unwrap(),
        Default::default(),
    ).expect("client");

    fn iter(client: &Client) {
        let (header, body) = client
            .start_get("/any", "localhost")
            .0
            .wait()
            .expect("headers");
        assert_eq!(200, header.status());

        let mut s = 0;
        for p in body.wait() {
            match p.expect("part") {
                DataOrTrailers::Data(d, ..) => s += d.len(),
                _ => panic!("unexpected headers"),
            }
        }

        assert_eq!(1024 * 1024, s);
    }

    // Warm-up
    iter(&client);

    b.iter(|| iter(&client))
}

#[bench]
fn small_requests(b: &mut Bencher) {
    struct My;

    impl ServerHandler for My {
        fn start_request(
            &self,
            _context: ServerHandlerContext,
            _req: ServerRequest,
            mut resp: ServerResponse,
        ) -> httpbis::Result<()> {
            resp.send_found_200_plain_text("hello there")?;
            Ok(())
        }
    }

    let mut server = ServerBuilder::new_plain();
    server.set_port(0);
    server.service.set_service("/", Arc::new(My));
    let server = server.build().expect("server");

    let client = Client::new_plain(
        "127.0.0.1",
        server.local_addr().port().unwrap(),
        Default::default(),
    ).expect("client");

    b.iter(|| {
        let (header, body) = client
            .start_get("/any", "localhost")
            .0
            .wait()
            .expect("headers");
        assert_eq!(200, header.status());

        // TODO: check content
        body.collect().wait().expect("body")
    })
}
