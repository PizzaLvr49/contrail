# Project Setup
default: setup

# Configures env, db, and sqlx-cli
setup: env db-up
    @if ! which -s sqlx; then just install-tools; fi
    @echo "waiting for db"
    sleep 5
    sqlx database setup

# Configures env
env:
    #!/usr/bin/env bash
    if [ ! -f .env ]; then \
        echo 'DATABASE_URL="postgres://postgres:1234@localhost:5432/contrail_dev"' > .env; \
        echo "created .env file"; \
    else \
        echo ".env file already exists"; \
    fi

# Runs the server
run-server:
    cargo run -p server

# Runs the client
run-client:
    cargo run -p client

# Sets up the db with password 1234 for local dev
db-up:
    docker run --name contrail-postgres-dev \
        -e POSTGRES_DB=contrail_dev \
        -e POSTGRES_USER=postgres \
        -e POSTGRES_PASSWORD=1234 \
        -p 5432:5432 \
        -d postgres:18-alpine

# Removes the db
db-down:
    docker rm -f contrail-postgres-dev

# Migrates the local db
migrate:
    sqlx migrate run

# Resets the local db
reset:
    sqlx database reset

# Prepares for type checked sqlx queries without a live db
db-prepare:
    cargo sqlx prepare --workspace -- --all-targets


# Installs sqlx-cli
install-tools:
    cargo install sqlx-cli