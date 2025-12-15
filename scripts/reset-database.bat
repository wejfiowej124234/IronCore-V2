@echo off
:: 完全重置数据库脚本（开发环境专用）
:: ⚠️ 警告：这会删除所有数据！仅用于开发环境
:: 使用方法: reset-database.bat

setlocal enabledelayedexpansion

echo.
echo ════════════════════════════════════════════════
echo   🗄️  CockroachDB 完全重置工具
echo ════════════════════════════════════════════════
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
echo ════════════════════════════════════════════════
echo 步骤 1/4: 查找并停止 CockroachDB 容器...
echo ════════════════════════════════════════════════

:: 查找并停止所有可能的容器
set FOUND=0

for %%c in (ironwallet-cockroachdb ironwallet-co cockroach) do (
    docker ps -a --filter "name=%%c" --format "{{.Names}}" >nul 2>&1
    if !errorlevel! equ 0 (
        for /f "tokens=*" %%i in ('docker ps -a --filter "name=%%c" --format "{{.Names}}" 2^>nul') do (
            if not "%%i"=="" (
                echo   ✓ 找到容器: %%i
                set FOUND=1
                echo   🛑 停止容器: %%i
                docker stop %%i >nul 2>&1
                echo   🗑️  删除容器: %%i
                docker rm %%i >nul 2>&1
                if !errorlevel! equ 0 (
                    echo     ✓ 已删除
                )
            )
        )
    )
)

if !FOUND! equ 0 (
    echo   ℹ️  未找到运行中的容器
)

echo.
echo ════════════════════════════════════════════════
echo 步骤 2/4: 查找并删除数据卷...
echo ════════════════════════════════════════════════

:: 查找并删除所有可能的数据卷
set FOUND=0

for %%v in (ops_crdb-data ironwallet_cockroachdb_crdb-data crdb-data) do (
    docker volume ls --filter "name=%%v" --format "{{.Name}}" >nul 2>&1
    if !errorlevel! equ 0 (
        for /f "tokens=*" %%i in ('docker volume ls --filter "name=%%v" --format "{{.Name}}" 2^>nul') do (
            if not "%%i"=="" (
                echo   ✓ 找到数据卷: %%i
                set FOUND=1
                echo   🗑️  删除数据卷: %%i
                docker volume rm %%i >nul 2>&1
                if !errorlevel! equ 0 (
                    echo     ✓ 已删除
                ) else (
                    echo     ⚠️  删除失败（可能正在使用中）
                )
            )
        )
    )
)

:: 也查找所有包含 crdb 的卷
for /f "tokens=*" %%i in ('docker volume ls --filter "name=crdb" --format "{{.Name}}" 2^>nul') do (
    echo   ✓ 找到数据卷: %%i
    set FOUND=1
    echo   🗑️  删除数据卷: %%i
    docker volume rm %%i >nul 2>&1
    if !errorlevel! equ 0 (
        echo     ✓ 已删除
    ) else (
        echo     ⚠️  删除失败（可能正在使用中）
    )
)

if !FOUND! equ 0 (
    echo   ℹ️  未找到数据卷
)

echo.
echo ════════════════════════════════════════════════
echo 步骤 3/4: 重新启动 CockroachDB 容器...
echo ════════════════════════════════════════════════

:: 获取脚本所在目录的父目录（项目根目录）
set "SCRIPT_DIR=%~dp0"
cd /d "%SCRIPT_DIR%.."

if not exist "ops\docker-compose.yml" (
    echo ❌ 未找到 docker-compose.yml 文件
    echo    请手动启动 CockroachDB 容器
    exit /b 1
)

echo   📁 项目目录: %CD%
echo   📄 Docker Compose: ops\docker-compose.yml
echo   🚀 启动容器...

cd ops
docker compose up -d cockroach
if errorlevel 1 (
    echo   ❌ 启动失败
    exit /b 1
)
cd ..

echo   ✓ 容器已启动

echo.
echo ════════════════════════════════════════════════
echo 步骤 4/4: 等待数据库就绪...
echo ════════════════════════════════════════════════

echo   ⏳ 等待数据库启动（最多60秒）...

set RETRY=0
set MAX_RETRY=30
set IS_READY=0

:wait_loop
if %RETRY% geq %MAX_RETRY% goto wait_done
set /a RETRY+=1

:: 检查容器状态
docker ps --filter "name=ironwallet-cockroachdb" --format "{{.Status}}" | findstr "Up" >nul 2>&1
if errorlevel 1 (
    timeout /t 2 /nobreak >nul
    goto wait_loop
)

:: 尝试连接数据库
docker exec ironwallet-cockroachdb cockroach sql --insecure -e "SELECT 1;" >nul 2>&1
if errorlevel 0 (
    set IS_READY=1
    goto wait_done
)

if %RETRY% equ 5 (
    echo     ... 等待中 (%RETRY%/%MAX_RETRY%) ...
)
if %RETRY% equ 10 (
    echo     ... 等待中 (%RETRY%/%MAX_RETRY%) ...
)
if %RETRY% equ 15 (
    echo     ... 等待中 (%RETRY%/%MAX_RETRY%) ...
)
if %RETRY% equ 20 (
    echo     ... 等待中 (%RETRY%/%MAX_RETRY%) ...
)
if %RETRY% equ 25 (
    echo     ... 等待中 (%RETRY%/%MAX_RETRY%) ...
)

timeout /t 2 /nobreak >nul
goto wait_loop

:wait_done
if %IS_READY% equ 1 (
    echo   ✓ 数据库已就绪！
) else (
    echo   ⚠️  数据库可能未完全就绪，但容器已启动
    echo      请稍后手动检查数据库状态
)

echo.
echo ════════════════════════════════════════════════
echo   ✅ 数据库重置完成！
echo ════════════════════════════════════════════════
echo.
echo 📋 下一步操作：
echo   1. 启动后端应用，迁移会自动执行
echo      命令: cargo run
echo.
echo   2. 或手动运行迁移脚本
echo      命令: scripts\run-migrations-cockroachdb.bat
echo.
echo   3. 检查数据库状态
echo      命令: docker ps --filter name=cockroach
echo.
echo 📊 数据库信息：
echo   • 容器名: ironwallet-cockroachdb
echo   • SQL 端口: localhost:26257
echo   • Admin UI: http://localhost:8090
echo.

