# kvdb_rs

A Rust rewrite of [kvdb](https://github.com/mrrfv/kvdb), a simple PostgreSQL-based key-value database server. It supports basic CRUD operations, and is nearly API-compatible with the original (with minor improvements that should not break existing clients).

## Features

- Very simple create, read, update, and delete (CRUD) API
- Meant to be directly connected to from the client, no additional backend required
- Optional key expiration after a specified duration of inactivity (incl. read operations)
- Read-only keys, which allow only read operations
- Key/value length limits
- CORS support

**Differences from the original kvdb:**
- No response speed throttling
