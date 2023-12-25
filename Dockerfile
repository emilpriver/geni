FROM rust:1.74.1 as builder
WORKDIR /usr/src/app
COPY . .

RUN cargo build --release

RUN cp target/release/geni /usr/src/app

FROM rust:1.71.0
COPY --from=builder /usr/src/app/aoc /usr/src/aoc/aoc
COPY --from=builder /usr/src/site /usr/src/aoc/target/site

WORKDIR /usr/src/geni

CMD ["./geni"]
