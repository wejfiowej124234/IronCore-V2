# IronCore Backend - Quick Start (PowerShell)
# 企业级多链钱包后端服务启动脚本

Write-Host ""
Write-Host "===============================================" -ForegroundColor Cyan
Write-Host "     IronCore Backend - Quick Start" -ForegroundColor Green
Write-Host "===============================================" -ForegroundColor Cyan
Write-Host ""

# 切换到 IronCore 目录
Set-Location $PSScriptRoot

# Step 1: 检查数据库
Write-Host "[1/3] Checking database..." -ForegroundColor Yellow
$dbRunning = docker ps | Select-String "ironwallet-cockroachdb"
if (-not $dbRunning) {
    Write-Host "     [WARN] CockroachDB not running, starting..." -ForegroundColor Yellow
    Set-Location ..
    docker compose -f ops/docker-compose.yml up -d cockroach redis
    Set-Location IronCore
    Write-Host "     [WAIT] Waiting 5 seconds for database..." -ForegroundColor Yellow
    Start-Sleep -Seconds 5
}
Write-Host "     [OK] Database ready" -ForegroundColor Green

# Step 2: 检查迁移状态
Write-Host ""
Write-Host "[2/3] Checking migrations..." -ForegroundColor Yellow
$migrationCount = docker exec ironwallet-cockroachdb ./cockroach sql --insecure --execute="SELECT COUNT(*) FROM ironcore._sqlx_migrations;" 2>&1 | Select-String "^\d+$" | Select-Object -Last 1
if ($migrationCount -match "(\d+)") {
    $count = [int]$matches[1]
    if ($count -lt 30) {
        Write-Host "     [WARN] Only $count migrations applied, expected 33" -ForegroundColor Yellow
        Write-Host "     [INFO] Run ./migrate_sqlx.sh in Git Bash to apply all migrations" -ForegroundColor Cyan
        Write-Host ""
        $response = Read-Host "Continue anyway? (y/n)"
        if ($response -ne "y") {
            exit
        }
    } else {
        Write-Host "     [OK] $count migrations applied" -ForegroundColor Green
    }
}

# Step 3: 启动后端
Write-Host ""
Write-Host "[3/3] Starting IronCore backend..." -ForegroundColor Yellow
Write-Host "     Port: 3012" -ForegroundColor Cyan
Write-Host "     Swagger: http://localhost:3012/swagger-ui" -ForegroundColor Cyan
Write-Host ""

# 设置环境变量
$env:SKIP_MIGRATIONS = "1"
$env:RUST_LOG = "info"

# 运行后端
cargo run --release --bin ironcore

