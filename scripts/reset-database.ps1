# PowerShell脚本：完全重置数据库（开发环境专用）
# ⚠️ 警告：这会删除所有数据！仅用于开发环境
# 使用方法: .\reset-database.ps1

param(
    [switch]$Force  # 跳过确认提示
)

$ErrorActionPreference = "Continue"

Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host "  🗄️  CockroachDB 完全重置工具" -ForegroundColor Cyan
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host ""
Write-Host "⚠️  ⚠️  ⚠️  警告：这将删除所有数据库数据！" -ForegroundColor Red
Write-Host "⚠️  仅用于开发环境！生产环境请勿使用！" -ForegroundColor Red
Write-Host ""

if (-not $Force) {
    $confirm = Read-Host "确认要重置数据库吗？输入 'YES' 继续"
    if ($confirm -ne "YES") {
        Write-Host "❌ 操作已取消" -ForegroundColor Yellow
        exit 0
    }
}

Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host "步骤 1/4: 查找并停止 CockroachDB 容器..." -ForegroundColor Yellow
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan

# 查找所有可能的容器名
$containerNames = @("ironwallet-cockroachdb", "ironwallet-co", "cockroach")
$foundContainers = @()

foreach ($name in $containerNames) {
    $container = docker ps -a --filter "name=$name" --format "{{.Names}}" 2>$null
    if ($container) {
        $foundContainers += $container
        Write-Host "  ✓ 找到容器: $container" -ForegroundColor Green
    }
}

if ($foundContainers.Count -eq 0) {
    Write-Host "  ℹ️  未找到运行中的容器" -ForegroundColor Gray
} else {
    # 停止所有找到的容器
    foreach ($container in $foundContainers) {
        Write-Host "  🛑 停止容器: $container" -ForegroundColor Yellow
        docker stop $container 2>$null | Out-Null
        if ($LASTEXITCODE -eq 0) {
            Write-Host "    ✓ 已停止" -ForegroundColor Green
        }
    }
    
    # 删除所有找到的容器
    foreach ($container in $foundContainers) {
        Write-Host "  🗑️  删除容器: $container" -ForegroundColor Yellow
        docker rm $container 2>$null | Out-Null
        if ($LASTEXITCODE -eq 0) {
            Write-Host "    ✓ 已删除" -ForegroundColor Green
        }
    }
}

Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host "步骤 2/4: 查找并删除数据卷..." -ForegroundColor Yellow
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan

# 查找所有可能的数据卷名
$volumeNames = @("ops_crdb-data", "ironwallet_cockroachdb_crdb-data", "crdb-data")
$foundVolumes = @()

foreach ($name in $volumeNames) {
    $volume = docker volume ls --filter "name=$name" --format "{{.Name}}" 2>$null
    if ($volume) {
        $foundVolumes += $volume
        Write-Host "  ✓ 找到数据卷: $volume" -ForegroundColor Green
    }
}

# 也查找所有包含 crdb 的卷
$allCrdbVolumes = docker volume ls --filter "name=crdb" --format "{{.Name}}" 2>$null
if ($allCrdbVolumes) {
    foreach ($vol in $allCrdbVolumes) {
        if ($foundVolumes -notcontains $vol) {
            $foundVolumes += $vol
            Write-Host "  ✓ 找到数据卷: $vol" -ForegroundColor Green
        }
    }
}

if ($foundVolumes.Count -eq 0) {
    Write-Host "  ℹ️  未找到数据卷" -ForegroundColor Gray
} else {
    # 删除所有找到的数据卷
    foreach ($volume in $foundVolumes) {
        Write-Host "  🗑️  删除数据卷: $volume" -ForegroundColor Yellow
        docker volume rm $volume 2>$null | Out-Null
        if ($LASTEXITCODE -eq 0) {
            Write-Host "    ✓ 已删除" -ForegroundColor Green
        } else {
            Write-Host "    ⚠️  删除失败（可能正在使用中）" -ForegroundColor Yellow
        }
    }
}

Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host "步骤 3/4: 重新启动 CockroachDB 容器..." -ForegroundColor Yellow
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan

# 获取项目根目录
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectRoot = Split-Path -Parent $scriptDir
$dockerComposePath = Join-Path $projectRoot "ops\docker-compose.yml"

if (-not (Test-Path $dockerComposePath)) {
    Write-Host "❌ 未找到 docker-compose.yml 文件: $dockerComposePath" -ForegroundColor Red
    Write-Host "   请手动启动 CockroachDB 容器" -ForegroundColor Yellow
    exit 1
}

Set-Location $projectRoot

Write-Host "  📁 项目目录: $projectRoot" -ForegroundColor Cyan
Write-Host "  📄 Docker Compose: ops\docker-compose.yml" -ForegroundColor Cyan
Write-Host "  🚀 启动容器..." -ForegroundColor Yellow

$composeResult = docker compose -f ops/docker-compose.yml up -d cockroach 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Host "  ❌ 启动失败:" -ForegroundColor Red
    Write-Host $composeResult
    exit 1
}

Write-Host "  ✓ 容器已启动" -ForegroundColor Green

Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host "步骤 4/4: 等待数据库就绪..." -ForegroundColor Yellow
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan

# 等待数据库启动并检查健康状态
$maxRetries = 30
$retryCount = 0
$isReady = $false

Write-Host "  ⏳ 等待数据库启动..." -ForegroundColor Cyan

while ($retryCount -lt $maxRetries -and -not $isReady) {
    Start-Sleep -Seconds 2
    $retryCount++
    
    # 检查容器是否运行
    $containerStatus = docker ps --filter "name=ironwallet-cockroachdb" --format "{{.Status}}" 2>$null
    if ($containerStatus -match "Up") {
        # 尝试连接数据库
        $dbCheck = docker exec ironwallet-cockroachdb cockroach sql --insecure -e "SELECT 1;" 2>$null
        if ($LASTEXITCODE -eq 0) {
            $isReady = $true
            Write-Host "  ✓ 数据库已就绪！" -ForegroundColor Green
            break
        }
    }
    
    if ($retryCount % 5 -eq 0) {
        Write-Host "    ... 等待中 ($retryCount/$maxRetries) ..." -ForegroundColor Gray
    }
}

if (-not $isReady) {
    Write-Host "  ⚠️  数据库可能未完全就绪，但容器已启动" -ForegroundColor Yellow
    Write-Host "     请稍后手动检查数据库状态" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Green
Write-Host "  ✅ 数据库重置完成！" -ForegroundColor Green
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Green
Write-Host ""
Write-Host "📋 下一步操作：" -ForegroundColor Cyan
Write-Host "   1. 启动后端应用，迁移会自动执行" -ForegroundColor White
Write-Host "     命令: cargo run" -ForegroundColor Gray
Write-Host ""
Write-Host "   2. 或手动运行迁移脚本" -ForegroundColor White
Write-Host "     命令: .\scripts\run-migrations-cockroachdb.bat" -ForegroundColor Gray
Write-Host ""
Write-Host "   3. 检查数据库状态" -ForegroundColor White
Write-Host "     命令: docker ps --filter name=cockroach" -ForegroundColor Gray
Write-Host ""
Write-Host "📊 数据库信息：" -ForegroundColor Cyan
Write-Host "   • 容器名: ironwallet-cockroachdb" -ForegroundColor White
Write-Host "   • SQL 端口: localhost:26257" -ForegroundColor White
Write-Host "   • Admin UI: http://localhost:8090" -ForegroundColor White
Write-Host ""

