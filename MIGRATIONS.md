# IronCore-V2 Migrations (Production-safe)

## Why

On Fly, running SQLx migrations during service startup can block the HTTP bind (e.g. advisory lock waits), which causes health checks to fail during rollouts.

Production approach:
- Keep `SKIP_MIGRATIONS=1` for the web service.
- Run migrations explicitly using the migration runner binary.

## Fix: duplicate migration versions

This repo historically had duplicate numeric migration versions (e.g. multiple `0004_*.sql`).
SQLx requires every migration version to be unique.

The duplicates are preserved as `*.sql.skip` for auditing, and canonical migrations are re-numbered to unique versions.

## Run migrations on Fly

1) Deploy (so the `ironcore_migrate` binary exists in the image).

2) Run migrations:

- Command:

  `flyctl ssh console -a oxidevault-ironcore-v2 -C "/usr/local/bin/ironcore_migrate"`

3) Verify:

- Check applied versions:

  `printf "SELECT MAX(version) FROM public._sqlx_migrations;\\n\\q\\n" | flyctl postgres connect -a oxidevault-ironcore-v2-db -d ironcore`

- Check health:

  `flyctl checks list -a oxidevault-ironcore-v2`
