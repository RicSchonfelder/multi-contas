@echo off
title Multi Contas
cd /d "%~dp0"

echo ========================================
echo    Multi Contas - Gerenciador de Perfis
echo ========================================
echo.

where npm >nul 2>&1
if %ERRORLEVEL% NEQ 0 (
    echo [ERRO] Node.js / npm nao encontrado. Instale de https://nodejs.org
    pause
    exit /b 1
)

echo [1/3] Instalando dependencias...
call npm exec -- pnpm install
if %ERRORLEVEL% NEQ 0 (
    echo [ERRO] Falha ao instalar. Tente: npm install -g pnpm
    pause
    exit /b 1
)

echo [2/3] Compilando...
call npm exec -- pnpm build 2>nul

echo [3/3] Iniciando Multi Contas...
echo.
echo A janela do app vai abrir em alguns segundos.
echo.
cd apps\desktop
npm exec -- pnpm tauri dev

pause
