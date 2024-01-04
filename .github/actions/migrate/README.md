# Geni migrations action

## Usage
```
- uses: emilpriver/geni/migrate
  with:
    migrations_folder: "./migrations"
    wait_timeout: "30"
    migrations_table: "schema_migrations"
  secrets:
    database_url: "https://localhost:3000"
    database_token: "X"
```

## Arguments

### With
  - `migrations_folder`(optional): The path to where your migrations exist.
    - Default: ./migrations
  - `wait_timeout`(optional): The time to wait before dropping the atempt to connect to the datbase
    - Default: 30
  - `migrations_table`(optional): The name of the migrations table
    - Default: `schema_migrations`

###  Secrets
  - database_url(required): The url for accessing your database
  - database_token(optional): The token used to authenticate towards Turso. Only needed if you need to authenticate yourself
    - Default: "" 

