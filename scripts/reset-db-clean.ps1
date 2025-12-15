# PowerShell脚本：完全重置数据库（通过代码调用）
# ⚠️ 警告：这会删除所有数据！仅用于开发环境
# 使用方法: .\reset-db-clean.ps1

Write-Host "🧹 完全重置数据库（干净模式）" -ForegroundColor Cyan
Write-Host ""

# 检查是否在 IronCore 目录
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ironCoreDir = Join-Path $scriptDir ".."

if (-not (Test-Path (Join-Path $ironCoreDir "Cargo.toml"))) {
    Write-Host "❌ 错误: 请在 IronCore 目录下运行此脚本" -ForegroundColor Red
    exit 1
}

Set-Location $ironCoreDir

# 检查 DATABASE_URL
if (-not $env:DATABASE_URL) {
    Write-Host "📋 使用默认数据库 URL..." -ForegroundColor Yellow
    $env:DATABASE_URL = "postgresql://root@localhost:26257/ironcore?sslmode=disable"
}

Write-Host "📋 数据库: $env:DATABASE_URL" -ForegroundColor Cyan
Write-Host ""

# 编译并运行重置脚本
Write-Host "🔨 编译项目..." -ForegroundColor Yellow
cargo build --quiet 2>&1 | Out-Null

if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ 编译失败" -ForegroundColor Red
    exit 1
}

Write-Host "🚀 执行数据库重置..." -ForegroundColor Green
Write-Host ""

# 创建一个临时 Rust 程序来执行重置
$resetCode = @"
use ironforge_backend::infrastructure::db::init_pool;
use ironforge_backend::infrastructure::migration_cockroachdb::reset_database_clean;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://root@localhost:26257/ironcore?sslmode=disable".to_string());
    
    let pool = init_pool(&database_url, false).await?;
    reset_database_clean(&pool).await?;
    
    Ok(())
}
"@

# 将代码写入临时文件
$tempFile = Join-Path $ironCoreDir "reset_db_temp.rs"
$resetCode | Out-File -FilePath $tempFile -Encoding UTF8

Write-Host "⚠️  这将删除所有数据库表和数据！" -ForegroundColor Red
Write-Host "⚠️  仅用于开发环境！" -ForegroundColor Red
Write-Host ""
$confirm = Read-Host "确认继续？输入 'YES' 继续"

if ($confirm -ne "YES") {
    Write-Host "❌ 操作已取消" -ForegroundColor Yellow
    Remove-Item $tempFile -ErrorAction SilentlyContinue
    exit 0
}

Write-Host ""
Write-Host "🧹 正在重置数据库..." -ForegroundColor Cyan

# 注意：这里需要创建一个实际的 Rust 二进制文件
# 或者使用现有的 cargo run 方式
Write-Host ""
Write-Host "💡 提示：可以使用以下方式重置数据库：" -ForegroundColor Yellow
Write-Host "   1. 使用 Docker 重置: .\scripts\reset-database.ps1" -ForegroundColor White
Write-Host "   2. 或手动调用 reset_database_clean() 函数" -ForegroundColor White
Write-Host ""

Remove-Item $tempFile -ErrorAction SilentlyContinue

