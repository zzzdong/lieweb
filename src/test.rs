use crate::*;
use bytes::buf::Buf;

#[test]
fn test_server() {
    let mut rt = tokio::runtime::Runtime::new().unwrap();

    const HELLO: &str = "hello, world!";

    rt.spawn(async move {
        tokio::time::delay_for(std::time::Duration::from_secs(1)).await;

        let client = hyper::client::Client::new();

        let resp = client
            .get(hyper::Uri::from_static("http://127.0.0.1:5000/hello"))
            .await
            .unwrap();

        let body = hyper::body::aggregate::<hyper::Response<hyper::Body>>(resp)
            .await
            .unwrap();

        assert_eq!(body.bytes(), HELLO.as_bytes());
    });

    rt.block_on(async {
        let mut server = App::new();
        server.get("/hello", |_req| async move { HELLO });
        let addr = "127.0.0.1:5000".parse().unwrap();
        server
            .run_with_shutdown(
                &addr,
                Some(tokio::time::delay_for(std::time::Duration::from_secs(3))),
            )
            .await
            .unwrap()
    });
}
