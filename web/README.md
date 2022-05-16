
## Testing

### Start frontend server for testing

```shell
trunk serve --proxy-backend=http://localhost:8081/api
```

### Run backend server for testing

```shell
cargo run --bin aba_server --release --features server,web-files
```
App page at: http://127.0.0.1:8080/

## Release 

### Run/Build release version

Before building backend server build wasm frontend:

```shell
pushd web
trunk build --release
popd
cargo clean
```

Run backend server with frontend included:
```shell
cargo run --bin aba_server --release --features server,web-files
```
Or build backend server binary with frontend included:
```shell
cargo build --bin aba_server --release --features server,web-files
target/release/aba_server
```

App page at: http://127.0.0.1:8081/

### Building on Mac OSX  

See `rust-bitcoin` [issue #254 instructions](https://github.com/rust-bitcoin/rust-secp256k1/pull/254#issuecomment-879588601):
```shell
brew install llvm
echo 'export PATH="/opt/homebrew/opt/llvm/bin:$PATH"' >> ~/.zshrc

CC=/usr/local/opt/llvm/bin/clang
AR=/usr/local/opt/llvm/bin/llvm-ar
```

But does not build yet on M1 systems, see [issue #283](https://github.com/rust-bitcoin/rust-secp256k1/issues/283#).