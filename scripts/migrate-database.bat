@echo off
:: 数据库迁移脚本 - 标准版本
:: 使用新的标准化迁移文件

setlocal enabledelayedexpansion

echo.
echo ════════════════════════════════════════════════
echo   🗄️  数据库迁移工具
echo ════════════════════════════════════════════════
echo.

:: 获取脚本所在目录的父目录（IronCore目录）
set "SCRIPT_DIR=%~dp0"
cd /d "%SCRIPT_DIR%.."

:: 检查DATABASE_URL环境变量
if not defined DATABASE_URL (
    if exist "config.toml" (
        :: 从 config.toml 读取数据库 URL（匹配 [database] 部分的 url）
        for /f "tokens=2 delims==" %%i in ('findstr /c:"url = " config.toml') do (
            set "DATABASE_URL=%%i"
            set "DATABASE_URL=!DATABASE_URL:"=!"
            set "DATABASE_URL=!DATABASE_URL: =!"
            :: 检查是否包含 postgresql:// 或 postgres://
            echo !DATABASE_URL! | findstr /i "postgres" >nul
            if errorlevel 1 (
                set "DATABASE_URL="
            ) else (
                goto :found_url
            )
        )
        :found_url
    )
    
    if not defined DATABASE_URL (
        echo [INFO] DATABASE_URL not found, using default
        set DATABASE_URL=postgresql://root@localhost:26257/ironcore?sslmode=disable
    )
fi

echo [INFO] Running database migrations...
echo [INFO] Database URL: %DATABASE_URL%
echo [INFO] Migrations directory: migrations
echo.

:: 检查sqlx是否安装
where sqlx >nul 2>&1
if errorlevel 1 (
    echo [ERROR] sqlx-cli not found in PATH
    echo [INFO] Please install: cargo install sqlx-cli
    echo [INFO] Or migrations will run automatically on backend startup
    exit /b 1
)

:: 使用sqlx migrate run
sqlx migrate run --database-url "%DATABASE_URL%"

if errorlevel 1 (
    echo.
    echo [WARN] Migration failed (non-fatal)
    echo [INFO] Backend will attempt to run migrations on startup
    echo [TIP] Check database connection and ensure CockroachDB is running
    exit /b 1
) else (
    echo.
    echo [OK] ✅ Migrations completed successfully!
    echo.
    echo [INFO] Migration files executed:
    echo    • 0001_schemas.sql - 创建 Schema
    echo    • 0002_core_tables.sql - 核心业务表
    echo    • 0003_gas_tables.sql - 费用系统表
    echo    • 0004_admin_tables.sql - 管理员表
    echo    • 0005_notify_tables.sql - 通知系统表
    echo    • 0006_asset_tables.sql - 资产聚合表
    echo    • 0007_tokens_tables.sql - 代币注册表
    echo    • 0008_events_tables.sql - 事件总线表
    echo    • 0009_fiat_tables.sql - 法币系统表
    echo    • 0010_constraints.sql - 外键和唯一约束
    echo    • 0011_indexes.sql - 索引
    echo    • 0012_check_constraints.sql - 检查约束
    echo    • 0013_initial_data.sql - 初始数据
    exit /b 0
)

