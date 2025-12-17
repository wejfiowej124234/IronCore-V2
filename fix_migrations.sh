#!/bin/bash

echo "========================================"
echo "IronCore-V2 Migration Fix Script"
echo "========================================"
echo ""

echo "[1/5] Terminating all database connections..."
printf "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = 'ironcore' AND pid <> pg_backend_pid();\n\\q\n" | flyctl postgres connect -a oxidevault-ironcore-v2-db -d ironcore
if [ $? -ne 0 ]; then
    echo "WARNING: Failed to terminate connections (may be empty)"
fi
echo "Done."
echo ""

echo "Waiting for database to stabilize..."
sleep 5
echo ""

echo "[2/5] Dropping migration tracking table..."
MAX_RETRIES=3
RETRY_COUNT=0
while [ $RETRY_COUNT -lt $MAX_RETRIES ]; do
    printf "DROP TABLE IF EXISTS public._sqlx_migrations CASCADE;\n\\q\n" | flyctl postgres connect -a oxidevault-ironcore-v2-db -d ironcore
    if [ $? -eq 0 ]; then
        break
    fi
    RETRY_COUNT=$((RETRY_COUNT + 1))
    if [ $RETRY_COUNT -lt $MAX_RETRIES ]; then
        echo "Retry $RETRY_COUNT/$MAX_RETRIES in 3 seconds..."
        sleep 3
    fi
done
if [ $RETRY_COUNT -eq $MAX_RETRIES ]; then
    echo "ERROR: Failed to drop table after $MAX_RETRIES attempts"
    exit 1
fi
echo "Done."
echo ""

echo "[3/5] Cleaning duplicate fee_collector_addresses..."
printf "DELETE FROM gas.fee_collector_addresses WHERE (chain, address, created_at) NOT IN (SELECT chain, address, MIN(created_at) FROM gas.fee_collector_addresses GROUP BY chain, address);\n\\q\n" | flyctl postgres connect -a oxidevault-ironcore-v2-db -d ironcore
echo "Done."
echo ""

echo "[4/5] Running migrations (this may take 2-3 minutes)..."
export MSYS2_ARG_CONV_EXCL='*'
flyctl ssh console -a oxidevault-ironcore-v2 -C "/usr/local/bin/ironcore_migrate"
if [ $? -ne 0 ]; then
    echo "WARNING: Migration returned error code, checking results..."
fi
echo ""

echo "[5/5] Verifying migration status..."
(printf "SELECT MAX(version) AS current_version, COUNT(*) AS total_migrations FROM public._sqlx_migrations;\n\\q\n") | flyctl postgres connect -a oxidevault-ironcore-v2-db -d ironcore
echo ""

echo "[FINAL] Checking application health..."
flyctl checks list -a oxidevault-ironcore-v2
echo ""

echo "========================================"
echo "Migration fix script completed!"
echo "========================================"
