FROM rust:1.75.0-alpine3.19 as builder
WORKDIR /usr/src/app
COPY . .

RUN apk add musl-dev

RUN cargo build --release

RUN cp target/release/geni /usr/src/app/geni

FROM alpine:3.19.1
COPY --from=builder /usr/src/app/geni /usr/src/app/geni

RUN apk add mariadb-client postgresql-client

ENV DATABASE_MIGRATIONS_FOLDER="/migrations"

LABEL org.opencontainers.image.description "Geni: Standalone database migration tool which works for Postgres, MariaDB, MySQL, Sqlite and LibSQL(Turso)."

WORKDIR /usr/src/app

ENTRYPOINT ["./geni"]
