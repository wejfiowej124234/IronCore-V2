@echo off
chcp 65001 >nul 2>&1

echo.
echo ===============================================
echo      IronCore Backend - Quick Start
echo ===============================================
echo.

cd /d "%~dp0"

:: Kill existing IronCore processes
echo [1/3] Cleaning up old processes...
taskkill /F /IM ironcore.exe >nul 2>&1
if errorlevel 1 (
    echo     [INFO] No existing processes found
) else (
    echo     [OK] Old processes terminated
)
echo     [WAIT] Waiting for ports to release...
timeout /t 2 /nobreak >nul

:: Check if database is running
echo.
echo [2/3] Checking database...
docker ps | findstr ironwallet-cockroachdb >nul 2>&1
if errorlevel 1 (
    echo     [WARN] CockroachDB not running, starting...
    cd ..
    docker compose -f ops\docker-compose.yml up -d cockroach redis
    cd IronCore
    timeout /t 5 /nobreak >nul
)
echo     [OK] Database ready

:: Start backend
echo.
echo [3/3] Starting IronCore backend...
echo     Port: 3012
echo     Swagger: http://localhost:3012/swagger-ui
echo     Database: postgresql://root@localhost:26257/ironcore?sslmode=disable
echo.

:: Set environment variables
rem SKIP_MIGRATIONS removed - migrations will run automatically
set RUST_LOG=info
set DATABASE_URL=postgresql://root@localhost:26257/ironcore?sslmode=disable
set REDIS_URL=redis://:cdoqUAJuktZ9y9U8tJ31v9hXCnUjxCjLlTxVsh7lavs=@localhost:6379
set JWT_SECRET=w5s85oUKczFa9cwGhx1LUYz9LGc9PMK34MzymMFe6Z6nvCchqkN9MYbCNUU3f6Ww
set WALLET_ENC_KEY=dev-wallet-encryption-key-32chars!!

echo     [INFO] Auto-migration enabled (first start may be slower)

:: Run backend
cargo run --release --bin ironcore

pause

