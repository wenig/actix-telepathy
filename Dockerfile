FROM rust:1.49
WORKDIR /app
COPY . .
RUN RUST_LOG=debug cargo t
