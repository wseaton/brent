# brent

## background

`brent` is an experimental tool for running SQL migrations in snowflake, very similar in functionality to schemachange and liquibase.

Why `brent`?

Snowflake -> Arctic

Refinery -> Oil

Arctic + Oil = [Brent Crude](https://en.wikipedia.org/wiki/Brent_Crude)

## installation

```sh
cargo install --path .
```

## usage

To run migrations in the `/.migrations` folder w/ the vault plugin enabled:

```sh
brent --path ./migrations --enable-vault
```

Required environment variables:

```ini
SNOWFLAKE_ACCOUNT
SNOWFLAKE_USER
SNOWFLAKE_DATABASE
SNOWFLAKE_ROLE
SNOWFLAKE_SCHEMA
SNOWFLAKE_PASSWORD
SNOWFLAKE_WAREHOUSE

VAULT_SECRET_ID
VAULT_ADDR
VAULT_ROLE_ID
```

## notable upstream deps

* refinery (migration framework) - <https://github.com/rust-db/refinery>
* minijinja (for templating) - <https://github.com/mitsuhiko/minijinja>
* minijinja-vault - <https://github.com/wseaton/minijinja-vault>

## versioning

brent uses [refinery](https://github.com/rust-db/refinery) under the hood for the logic that handles migrations

## TODO

* [ ] support vault for snowflake credentials
* [ ] more jinja plugins? custom filters?
* [ ] distribute minimal container image
