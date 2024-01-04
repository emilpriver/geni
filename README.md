# Geni

Geni is a standalone migration tool designed to work in conjunction with your preferred ORM/toolkit/code. It allows multiple developers to collaborate without overriding database migrations. It can be used in a CD pipeline alongside your code to ensure your database stays up to date.

This project was heavily inspired by [dbmate](https://github.com/amacneil/dbmate) and was created because dbmate lacked support for LibSQL.

The application is developed using the Rust programming language and relies on the [libsql-client-rs](https://github.com/libsql/libsql-client-rs) library for SQLite and LibSQL. Moreover, it makes use of [SQLX](https://github.com/launchbadge/sqlx) to support Postgres, MariaDB, and MySQL databases. As this is written in rust is lighting fast, blazingly fast, tiny, ultra fast and memory safe

## Features

- Databases:
    - Postgres
    - MariaDB
    - MySQL
    - SQLite
    - LibSQL
- Generating migrations using `geni new **name**`
- Migrating using `geni up`
- Rollback using `geni down`
- Create database using  `geni create`
- Dropping database using  `geni drop`
- Timestamp based migrations
- Running migrations in a transaction
- Status command to see which migrations that is pending to be applied 
- Dump a schema.sql after each migration which can be used in version control
  - Dumping needs another binaries to work:
    - Postgres: pg_dump need to be installed(already instead in docker)
    - MySQL: mysqldump need to be installed(already instead in docker)
    - MariaDB: mariadb-dump need to be installed(already instead in docker)
    - Sqlite: Works without need for another binary. Uses SQL code get schema
    - LibSQL: Works without need for another binary. Uses SQL code get schema

## TODO

- [ ]  Databases
    - [ ]  ClickHouse

## Installation

### Github

```
$ sudo curl -fsSL -o /usr/local/bin/geni https://github.com/emilpriver/geni/releases/latest/download/geni-linux-amd64
$ sudo chmod +x /usr/local/bin/geni
```

### Homebrew

```
brew install emilpriver/geni/geni
```

### Scoop

TBA

### PKGX
Run using PKGX
```
pkgx geni up
```

### Cargo

```
cargo install geni
```

### Docker

Docker images are published to GitHub Container Registry ([ghcr.io/emilpriver/geni](https://ghcr.io/emilpriver/geni)).

```
$ docker run --rm -it --network=host ghcr.io/emilpriver/geni:latest --help
```

If you wish to create or apply migrations, you will need to use Docker's [bind mount](https://docs.docker.com/storage/bind-mounts/) feature to make your local working directory (`pwd`) available inside the geni container:

```
$ docker run --rm -it --network=host -v "$(pwd)/migrations:/migrations" ghcr.io/emilpriver/geni:latest new create_users_table`
```

### Commands

```rust
geni new    # Generate a new migrations file
geni up     # Run any pending migration
geni down   # Rollback migrations, use --amount to speify how many migrations(default 1)
geni create # Create the database, only works for Postgres, MariaDB and MySQL. If you use SQLite will geni create the file before running migrations if the sqlite file don't exist. LibSQL should be create using respective interface.
geni drop   # Remove database
geni status # Print pending migrations
geni help   # Print help message
```

## Environment variables

- `DATABASE_MIGRATIONS_FOLDER`
    - Specify where geni should look for migrations to run.
    - Default: `./migrations`
- `DATABASE_URL`
    - The database url geni should use to make migrations
    - Examples:
        - Postgres:`DATABASE_URL="postgres://postgres@127.0.0.1:5432/app?sslmode=disable"`
        - MySQL: `mysql://root:password@localhost:3307/app`
        - MariaDB: `mariadb://root:password@localhost:3307/app`
        - Sqlite: `sqlite://./database.sqlite`
        - LibSQL: `https://localhost:6000`
            - The protocol for LibSQL is https.
            - For turso uses: This is something you can retrieve using Turso CLI or the website
- `DATABASE_TOKEN`
    - Only if you use `Turso` and `LibSQL` and require token to authenticate. If not specified will Geni try to migrate without any auth
- `DATABASE_WAIT_TIMEOUT`
    - Time for geni to wait before trying to migrate. Useful if your database need some time to boot
    - Default: `0` seconds
- `DATABASE_SCHEMA_FILE`
  - Name of the schema migration file
- `DATABASE_MIGRATIONS_TABLE`
  - Name of the table to run migrations to
## Usage

### Creating a new migration

Running 

```rust
DATABASE_URL="x" geni new hello_world
```

Will create 2 files(with path written in console). 1 file ending with `.up.sql` and 1 ending with `.down.sql`. `.up.sql` is for creating migrations and `.down.sql` is for rollbacking migrations. This means that  `.down.sql` should include information that rollback the changed you added to `.up.sql`. 

Example:

If I want to create  table named `Persons` should I add this to `.up.sql`

```rust
CREATE TABLE Persons (
    PersonID int
)
```

And the rollback migration should then be

```rust
DROP TABLE Persons;
```

in the generated `.down.sql` file as this code would revert the creation of the table `Persons`

### Running migration

Running migration can be done using

```rust
geni up
```

### Rollback migrations

Rollbacking last added migrations can be done using

```rust
geni down
```

and If you want to rollback more then 1 can you add `-a` to the cli to specify how many

```rust
geni down -a 3
```

### Running from CLI

```rust
DATABASE_URL="postgres://postgres@127.0.0.1:5432/app?sslmode=disable" geni up
```

### Github Workflow

TBA

### Running in CI/CD

In a CI/CD should the database_url come from a seurity store and be appended to the environment as the `DATABASE_URL`. If the `DATABASE_URL` is provided as a environment variable is the only command you need to run

```rust
geni up
```

to make migrations.
