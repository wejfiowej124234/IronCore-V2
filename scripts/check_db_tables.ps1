# 数据库表完整性检查脚本
Write-Host "=== 数据库迁移完整性检查 ===" -ForegroundColor Green
Write-Host ""

# 统计迁移文件
$migrations = Get-ChildItem -Path migrations -Filter *.sql | Sort-Object Name
Write-Host "迁移文件数量: $($migrations.Count)" -ForegroundColor Cyan
Write-Host ""

# 提取所有CREATE TABLE语句
Write-Host "扫描表定义..." -ForegroundColor Yellow
$tables = @{}

foreach ($file in $migrations) {
    $content = Get-Content $file.FullName -Raw -ErrorAction SilentlyContinue
    if ($content) {
        $matches = [regex]::Matches($content, 'CREATE\s+TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?(?:\w+\.)?(\w+)')
        foreach ($match in $matches) {
            $tableName = $match.Groups[1].Value
            if (-not $tables.ContainsKey($tableName)) {
                $tables[$tableName] = $file.Name
            }
        }
    }
}

Write-Host "找到 $($tables.Count) 个表定义" -ForegroundColor Green
Write-Host ""

# 显示所有表
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host "所有表列表:" -ForegroundColor Yellow
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
$tables.Keys | Sort-Object | ForEach-Object {
    Write-Host "  • $_" -ForegroundColor White
}

# 检查关键表
Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host "关键表检查:" -ForegroundColor Yellow
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan

$criticalTables = @(
    'tenants',
    'users',
    'wallets',
    'transactions',
    'wallet_unlock_tokens',
    'audit_logs',
    'fee_configurations',
    'rpc_endpoints',
    'nonce_tracking',
    'broadcast_queue',
    'platform_addresses',
    'fiat_orders',
    'cross_chain_transactions',
    'tokens',
    'assets',
    'notifications',
    'sessions'
)

$missing = @()
foreach ($table in $criticalTables) {
    if ($tables.ContainsKey($table)) {
        Write-Host "  ✅ $table" -ForegroundColor Green
    } else {
        Write-Host "  ❌ $table (缺失)" -ForegroundColor Red
        $missing += $table
    }
}

# 检查非托管关键迁移
Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host "非托管关键迁移检查:" -ForegroundColor Yellow
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan

$keyMigrations = @{
    '0030_remove_custodial_features.sql' = '删除托管功能'
    '0035_wallet_unlock_tokens.sql' = '双锁机制'
    '0039_non_custodial_compliance_checks.sql' = '合规性检查'
}

foreach ($migration in $keyMigrations.Keys) {
    if (Test-Path "migrations\$migration") {
        Write-Host "  ✅ $migration - $($keyMigrations[$migration])" -ForegroundColor Green
    } else {
        Write-Host "  ❌ $migration - $($keyMigrations[$migration])" -ForegroundColor Red
    }
}

# 总结
Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host "检查总结:" -ForegroundColor Yellow
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host ""
Write-Host "迁移文件: $($migrations.Count) 个" -ForegroundColor Cyan
Write-Host "表定义: $($tables.Count) 个" -ForegroundColor Cyan
Write-Host "缺失关键表: $($missing.Count) 个" -ForegroundColor $(if ($missing.Count -eq 0) { "Green" } else { "Red" })

if ($missing.Count -eq 0) {
    Write-Host ""
    Write-Host "🎉 数据库迁移完整性检查通过！" -ForegroundColor Green
    Write-Host ""
    Write-Host "可以执行迁移:" -ForegroundColor Cyan
    Write-Host "  .\apply_migrations_cargo.ps1" -ForegroundColor White
} else {
    Write-Host ""
    Write-Host "⚠️  发现缺失的关键表，需要补充迁移文件" -ForegroundColor Yellow
}

Write-Host ""

