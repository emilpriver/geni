FROM rust:1.74.1 as builder
WORKDIR /usr/src/app
COPY . .

RUN cargo build --release

RUN cp target/release/geni /usr/src/app

FROM rust:1.71.0
COPY --from=builder /usr/src/app/geni /usr/src/app

WORKDIR /usr/src/app

CMD ["./geni"]
