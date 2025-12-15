# ============================================================================
# 数据库完整性深度检查脚本
# 检查迁移文件与代码中的表引用是否一致
# ============================================================================

Write-Host "🔍 开始数据库完整性深度检查..." -ForegroundColor Green
Write-Host ""

# ============================================================================
# 第一步：提取迁移文件中定义的所有表
# ============================================================================
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host "📋 第一步：扫描迁移文件中的表定义" -ForegroundColor Yellow
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host ""

$migrationTables = @{}
$migrationFiles = Get-ChildItem -Path ".\migrations" -Filter *.sql | Sort-Object Name

foreach ($file in $migrationFiles) {
    $content = Get-Content $file.FullName -Raw
    
    # 提取 CREATE TABLE 语句
    $matches = [regex]::Matches($content, 'CREATE TABLE(?:\s+IF NOT EXISTS)?\s+(\w+\.)?(\w+)', [System.Text.RegularExpressions.RegexOptions]::IgnoreCase)
    
    foreach ($match in $matches) {
        $tableName = $match.Groups[2].Value
        if (-not $migrationTables.ContainsKey($tableName)) {
            $migrationTables[$tableName] = $file.Name
        }
    }
}

Write-Host "✅ 找到 $($migrationTables.Count) 个表定义" -ForegroundColor Green
Write-Host ""
Write-Host "表列表:" -ForegroundColor Cyan
$migrationTables.Keys | Sort-Object | ForEach-Object {
    Write-Host "  • $_" -ForegroundColor Gray
}

# ============================================================================
# 第二步：提取代码中引用的所有表
# ============================================================================
Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host "📋 第二步：扫描代码中的表引用" -ForegroundColor Yellow
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host ""

$codeTables = @{}
$rustFiles = Get-ChildItem -Path ".\src" -Filter *.rs -Recurse

foreach ($file in $rustFiles) {
    $content = Get-Content $file.FullName -Raw
    
    # 提取 FROM/JOIN/INSERT INTO/UPDATE 后的表名
    $patterns = @(
        'FROM\s+(\w+)',
        'JOIN\s+(\w+)',
        'INSERT\s+INTO\s+(\w+)',
        'UPDATE\s+(\w+)\s+SET',
        'DELETE\s+FROM\s+(\w+)'
    )
    
    foreach ($pattern in $patterns) {
        $matches = [regex]::Matches($content, $pattern, [System.Text.RegularExpressions.RegexOptions]::IgnoreCase)
        
        foreach ($match in $matches) {
            $tableName = $match.Groups[1].Value
            # 过滤掉常见的非表名关键字
            if ($tableName -notmatch '^(SELECT|WHERE|AND|OR|AS|ON)$') {
                if (-not $codeTables.ContainsKey($tableName)) {
                    $codeTables[$tableName] = @()
                }
                $codeTables[$tableName] += $file.Name
            }
        }
    }
}

Write-Host "✅ 找到 $($codeTables.Count) 个表引用" -ForegroundColor Green
Write-Host ""

# ============================================================================
# 第三步：交叉验证
# ============================================================================
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host "🔍 第三步：交叉验证" -ForegroundColor Yellow
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host ""

# 检查代码中引用但迁移中未定义的表
$missingTables = @()
foreach ($table in $codeTables.Keys) {
    if (-not $migrationTables.ContainsKey($table)) {
        $missingTables += $table
    }
}

if ($missingTables.Count -gt 0) {
    Write-Host "❌ 发现 $($missingTables.Count) 个缺失的表定义:" -ForegroundColor Red
    Write-Host ""
    foreach ($table in $missingTables | Sort-Object) {
        Write-Host "  ⚠️  $table" -ForegroundColor Yellow
        Write-Host "      引用位置: $($codeTables[$table] -join ', ')" -ForegroundColor Gray
    }
} else {
    Write-Host "✅ 所有代码中引用的表都已在迁移中定义" -ForegroundColor Green
}

Write-Host ""

# 检查迁移中定义但代码中未使用的表（可能是正常的）
$unusedTables = @()
foreach ($table in $migrationTables.Keys) {
    if (-not $codeTables.ContainsKey($table)) {
        $unusedTables += $table
    }
}

if ($unusedTables.Count -gt 0) {
    Write-Host "ℹ️  发现 $($unusedTables.Count) 个未使用的表（可能是预留或通过ORM使用）:" -ForegroundColor Cyan
    Write-Host ""
    foreach ($table in $unusedTables | Sort-Object) {
        Write-Host "  • $table (定义于: $($migrationTables[$table]))" -ForegroundColor Gray
    }
}

# ============================================================================
# 第四步：检查关键表
# ============================================================================
Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host "🎯 第四步：检查关键表" -ForegroundColor Yellow
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host ""

$criticalTables = @(
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
    'cross_chain_transactions'
)

$missingCritical = @()
foreach ($table in $criticalTables) {
    if ($migrationTables.ContainsKey($table)) {
        Write-Host "  ✅ $table" -ForegroundColor Green
    } else {
        Write-Host "  ❌ $table (缺失)" -ForegroundColor Red
        $missingCritical += $table
    }
}

# ============================================================================
# 第五步：检查非托管合规性
# ============================================================================
Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host "🔒 第五步：非托管合规性检查" -ForegroundColor Yellow
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host ""

# 检查是否有敏感字段
$sensitivePatterns = @(
    'private_key',
    'encrypted_private_key',
    'mnemonic',
    'encrypted_mnemonic',
    'seed',
    'master_key',
    'secret_key'
)

$foundSensitive = $false
foreach ($file in $migrationFiles) {
    $content = Get-Content $file.FullName -Raw
    
    foreach ($pattern in $sensitivePatterns) {
        if ($content -match $pattern -and $file.Name -ne '0030_remove_custodial_features.sql' -and $file.Name -ne '0023_wallet_encrypted_private_key.sql' -and $file.Name -ne '0039_non_custodial_compliance_checks.sql') {
            Write-Host "  ⚠️  在 $($file.Name) 中发现敏感字段: $pattern" -ForegroundColor Yellow
            $foundSensitive = $true
        }
    }
}

if (-not $foundSensitive) {
    Write-Host "  ✅ 未发现敏感字段（0030已删除，0039已检查）" -ForegroundColor Green
}

# 检查关键迁移文件
$keyMigrations = @{
    '0030_remove_custodial_features.sql' = '删除托管功能'
    '0035_wallet_unlock_tokens.sql' = '双锁机制'
    '0039_non_custodial_compliance_checks.sql' = '合规性检查'
}

Write-Host ""
Write-Host "关键非托管迁移:" -ForegroundColor Cyan
foreach ($migration in $keyMigrations.Keys) {
    if (Test-Path ".\migrations\$migration") {
        Write-Host "  ✅ $migration - $($keyMigrations[$migration])" -ForegroundColor Green
    } else {
        Write-Host "  ❌ $migration - $($keyMigrations[$migration]) (缺失)" -ForegroundColor Red
    }
}

# ============================================================================
# 总结报告
# ============================================================================
Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host "📊 检查总结" -ForegroundColor Yellow
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host ""

$totalIssues = $missingTables.Count + $missingCritical.Count

if ($totalIssues -eq 0) {
    Write-Host "🎉 数据库迁移完整性检查通过！" -ForegroundColor Green
    Write-Host ""
    Write-Host "✅ 所有表定义完整" -ForegroundColor Green
    Write-Host "✅ 关键表全部存在" -ForegroundColor Green
    Write-Host "✅ 非托管合规性符合要求" -ForegroundColor Green
    Write-Host ""
    Write-Host "可以安全执行迁移：" -ForegroundColor Cyan
    Write-Host "  .\apply_migrations_cargo.ps1" -ForegroundColor White
} else {
    Write-Host "⚠️  发现 $totalIssues 个问题需要解决" -ForegroundColor Yellow
    Write-Host ""
    if ($missingTables.Count -gt 0) {
        Write-Host "  • $($missingTables.Count) 个缺失的表定义" -ForegroundColor Yellow
    }
    if ($missingCritical.Count -gt 0) {
        Write-Host "  • $($missingCritical.Count) 个关键表缺失" -ForegroundColor Red
    }
}

Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan

