# CONTRIBUTING

## Requirements
- Docker 
- Docker Compose
- Rust

## Setup

1. Fork this repositority and clone it to your machine and enter the folder
2. `docker compose up` to start the databases, keep docker running. Otherwise won't you be able to run the commands
3. `DATABASE_URL=x cargo run status` to verify that you have access to the database

## Selecting database url 

1. SQlite: `sqlite://temp.sqlite`
2. Postgres: `psql://postgres:mysecretpassword@localhost:6437/development?sslmode=disable`
3. MySQL: `mysql://root:password@localhost:3306/app`
4. MariaDB: `mariadb://root:password@localhost:3307/app`
5. LibSQL: `http://localhost:6000`


## Testing
To help with running the integration tests do you run

```make test```

which will start docker databases and running the tests and when it's done will it make a tiny clean up
