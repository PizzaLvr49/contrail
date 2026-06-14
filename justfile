set shell := ["bash", "-cu"]

CONTAINER := "contrail-postgres-dev"
DB_URL := "postgres://postgres:1234@localhost:5432/contrail_dev"

# Lists Commands
default:
    @just --list

# Configures env, db, and sqlx-cli
setup: env install-tools db-up
    @echo "Waiting for PostgreSQL..."
    @until docker exec {{CONTAINER}} pg_isready -U postgres >/dev/null 2>&1; do \
        sleep 1; \
    done
    DATABASE_URL={{DB_URL}} sqlx database setup

# Creates .env if it doesn't exist
env:
    @if [ ! -f .env ]; then \
        echo 'DATABASE_URL="{{DB_URL}}"' > .env; \
        echo "Created .env"; \
    else \
        echo ".env already exists"; \
    fi

# Runs the server
run-server *args:
    cargo run -p server -- {{args}}

# Runs the client
run-client *args:
    cargo run -p client -- {{args}}

# Starts the local PostgreSQL container
db-up:
    @if docker ps -a --format '{{"{{"}}.Names{{"}}"}}' | grep -qx {{CONTAINER}}; then \
        echo "Container already exists, starting it..."; \
        docker start {{CONTAINER}} >/dev/null; \
    else \
        docker run \
            --name {{CONTAINER}} \
            -e POSTGRES_DB=contrail_dev \
            -e POSTGRES_USER=postgres \
            -e POSTGRES_PASSWORD=1234 \
            -p 5432:5432 \
            -d postgres:18-alpine >/dev/null; \
    fi

# Stops and removes the local database
db-down:
    -docker rm -f {{CONTAINER}}

# Runs migrations
migrate:
    sqlx migrate run

# Resets the local database
reset:
    sqlx database reset

# Generates sqlx offline metadata
db-prepare:
    cargo sqlx prepare --workspace -- --all-targets

# Installs sqlx-cli if missing
install-tools:
    @if ! command -v sqlx >/dev/null 2>&1; then \
        cargo install sqlx-cli; \
    fi