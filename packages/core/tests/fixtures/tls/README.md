# Test-only TLS fixtures

`cert.der` and `key.der` are a self-signed certificate and its private key for `localhost`
(the tests serve it on 127.0.0.1). They exist only so `tests/tls_server.rs` can run a local TLS
server that the client refuses. Nothing outside `packages/core/tests/` reads them, and they protect
nothing.
