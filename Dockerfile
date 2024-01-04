FROM rust:1.75.0 as builder
WORKDIR /usr/src/app
COPY . .

RUN cargo build --release

RUN cp target/release/geni /usr/src/app/geni

FROM ubuntu:24.04
COPY --from=builder /usr/src/app/geni /usr/src/app/geni

RUN apt-get update && apt-get install -y mysql-client mariadb-client postgresql-client postgresql-client-common libpq-dev

WORKDIR /usr/src/app

CMD ["./geni"]
