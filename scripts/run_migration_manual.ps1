# PowerShell脚本：手动运行数据库迁移（适用于CockroachDB）
# 使用方法: .\run_migration_manual.ps1

Write-Host "🚀 开始运行数据库迁移..." -ForegroundColor Green

# 检查DATABASE_URL环境变量
if (-not $env:DATABASE_URL) {
    Write-Host "❌ 错误: DATABASE_URL环境变量未设置" -ForegroundColor Red
    Write-Host "请设置: `$env:DATABASE_URL='postgres://root@localhost:26257/ironcore?sslmode=disable'" -ForegroundColor Yellow
    exit 1
}

Write-Host "📋 使用数据库: $env:DATABASE_URL" -ForegroundColor Cyan

# 运行sqlx migrate
Write-Host "📦 执行迁移..." -ForegroundColor Yellow
sqlx migrate run --database-url $env:DATABASE_URL

if ($LASTEXITCODE -eq 0) {
    Write-Host "✅ 迁移完成!" -ForegroundColor Green
} else {
    Write-Host "❌ 迁移失败!" -ForegroundColor Red
    exit 1
}

