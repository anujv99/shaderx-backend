FROM rust:latest

RUN USER=root cargo new --bin backend-app
WORKDIR /backend-app

COPY . .

RUN cargo build --release

COPY --from=builder /backend-app/target/release/backend-app /usr/local/bin/backend-app

CMD ["backend-app"]

