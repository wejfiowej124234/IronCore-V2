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
echo     Port: 8088
echo     Docs: http://localhost:8088/docs/
echo     OpenAPI: http://localhost:8088/openapi.yaml
echo     Database: postgresql://root@localhost:26257/ironcore?sslmode=disable
echo.

:: Set environment variables
rem NOTE: For local dev, you may set SKIP_MIGRATIONS=1 to speed up startup.
set RUST_LOG=info
set DATABASE_URL=postgresql://root@localhost:26257/ironcore?sslmode=disable
set REDIS_URL=redis://localhost:6379
rem Dev-only defaults. Override these in your environment or .env.
if "%JWT_SECRET%"=="" set JWT_SECRET=dev-jwt-secret-change-me-in-production-32chars
if "%WALLET_ENC_KEY%"=="" set WALLET_ENC_KEY=dev-wallet-enc-key-change-me-32chars!!

echo     [INFO] If first start is slow, consider setting SKIP_MIGRATIONS=1

:: Run backend
cargo run --release --bin ironcore

pause

