@echo off
:: 简单重置数据库脚本 - 通过环境变量触发
:: ⚠️ 警告：这会删除所有数据！仅用于开发环境

setlocal enabledelayedexpansion

echo.
echo ⚠️  ⚠️  ⚠️  警告：这将删除所有数据库数据！
echo ⚠️  仅用于开发环境！生产环境请勿使用！
echo.
set /p confirm="确认要重置数据库吗？输入 YES 继续: "

if /i not "%confirm%"=="YES" (
    echo ❌ 操作已取消
    exit /b 0
)

echo.
echo 🧹 正在重置数据库...
echo.

:: 设置环境变量触发重置
set RESET_DB=true

:: 获取脚本所在目录
set "SCRIPT_DIR=%~dp0"
cd /d "%SCRIPT_DIR%.."

:: 运行后端（会自动重置数据库）
echo 🚀 启动后端并重置数据库...
echo    注意：后端启动后会自动重置数据库并退出
echo.

cargo run --profile release-fast

echo.
echo ✅ 数据库重置完成！
echo.

