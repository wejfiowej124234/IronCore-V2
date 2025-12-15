# 测试重置脚本 - 检查环境是否就绪
# 不执行实际重置，只检查环境

Write-Host "🔍 检查 Docker 重置环境..." -ForegroundColor Cyan
Write-Host ""

# 检查 Docker 是否运行
Write-Host "1. 检查 Docker..." -ForegroundColor Yellow
$dockerRunning = docker ps 2>$null
if ($LASTEXITCODE -ne 0) {
    Write-Host "   ❌ Docker 未运行或未安装" -ForegroundColor Red
    exit 1
}
Write-Host "   ✓ Docker 正在运行" -ForegroundColor Green

# 检查 docker-compose
Write-Host "2. 检查 docker-compose..." -ForegroundColor Yellow
$composeVersion = docker compose version 2>$null
if ($LASTEXITCODE -ne 0) {
    Write-Host "   ❌ docker-compose 不可用" -ForegroundColor Red
    exit 1
}
Write-Host "   ✓ docker-compose 可用: $($composeVersion -split "`n" | Select-Object -First 1)" -ForegroundColor Green

# 检查 docker-compose.yml
Write-Host "3. 检查 docker-compose.yml..." -ForegroundColor Yellow
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectRoot = Split-Path -Parent $scriptDir
$dockerComposePath = Join-Path $projectRoot "ops\docker-compose.yml"

if (Test-Path $dockerComposePath) {
    Write-Host "   ✓ 找到 docker-compose.yml: $dockerComposePath" -ForegroundColor Green
} else {
    Write-Host "   ❌ 未找到 docker-compose.yml" -ForegroundColor Red
    exit 1
}

# 检查现有容器
Write-Host "4. 检查现有容器..." -ForegroundColor Yellow
$containers = docker ps -a --filter "name=cockroach" --format "{{.Names}}" 2>$null
if ($containers) {
    Write-Host "   找到容器:" -ForegroundColor Cyan
    foreach ($container in $containers) {
        $status = docker ps -a --filter "name=$container" --format "{{.Status}}" 2>$null
        Write-Host "     • $container ($status)" -ForegroundColor White
    }
} else {
    Write-Host "   ℹ️  未找到 CockroachDB 容器" -ForegroundColor Gray
}

# 检查现有数据卷
Write-Host "5. 检查现有数据卷..." -ForegroundColor Yellow
$volumes = docker volume ls --filter "name=crdb" --format "{{.Name}}" 2>$null
if ($volumes) {
    Write-Host "   找到数据卷:" -ForegroundColor Cyan
    foreach ($volume in $volumes) {
        Write-Host "     • $volume" -ForegroundColor White
    }
} else {
    Write-Host "   ℹ️  未找到 CockroachDB 数据卷" -ForegroundColor Gray
}

Write-Host ""
Write-Host "✅ 环境检查完成！" -ForegroundColor Green
Write-Host ""
Write-Host "📋 下一步：" -ForegroundColor Cyan
Write-Host "   运行重置脚本: .\reset-database.ps1" -ForegroundColor White

