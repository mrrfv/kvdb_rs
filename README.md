# kvdb_rs

A Rust rewrite of [kvdb](https://github.com/mrrfv/kvdb), a simple PostgreSQL-based key-value database server. It supports basic CRUD operations, and is API-compatible with the original (with minor improvements that should not break existing clients).

## Features

- Very straightforward create, read, update, and delete (CRUD) API
- Predictable JSON input and output with error handling
- Meant to be directly connected to from the client, no additional backend required
- Optional key expiration after a specified duration of inactivity (incl. read operations)
- Read-only keys, which allow only read operations
- Key/value length limits
- CORS support

**Differences from the original kvdb:**
- No response speed throttling

## API documentation

Swagger is available at the `/` endpoint.

## Installation

1. Download the latest release from the [releases page](https://github.com/mrrfv/kvdb_rs/releases), use the provided Dockerfile, or build from source like an ordinary Rust project.

2. Copy `.env.example` to `.env`

3. Edit `.env` to set the database connection URL, CORS origins, and other settings. Of course, you can also set these as environment variables, and this assumes the PostgreSQL database is already set up and running.

4. Run the server with:
    - `./kvdb_rs` if you downloaded the binary
    - `cargo run` if building from source
    - `docker build -t kvdb_rs . && docker run -p 3005:3005 --env-file .env kvdb_rs` if using Docker

The server will listen on port 3005 by default, but you can change this by setting the `LISTEN_ON` environment variable.

## License

kvdb_rs is licensed under the GNU Affero General Public License v3.0 (AGPL-3.0).
