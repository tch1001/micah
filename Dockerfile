FROM rust:1.79.0

COPY . .

RUN cargo build --bin process --release
RUN cargo build --bin web --release
RUN cargo build --bin listener --release