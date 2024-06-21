FROM rust:1-bullseye as builder
WORKDIR /code
COPY . .
RUN cargo build --release  

FROM debian:bullseye-slim
RUN apt-get update && apt-get install 
COPY --from=builder /code/target/release/xmtp-message-history-server /usr/local/bin/xmtp-message-history-server
ENV RUST_LOG=info
CMD ["xmtp-message-history-server"]
