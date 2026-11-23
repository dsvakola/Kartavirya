@echo off
echo ========================================================
echo   Kartavirya - Reminder Companion
echo   Build Automation Script (Vidyasagar Academy)
echo ========================================================
echo.

set PROJECT_DIR=%~dp0
cd /d "%PROJECT_DIR%"

echo [1/4] Closing any running instances and cleaning previous Release artifacts...
taskkill /f /im portable.exe 2>nul
taskkill /f /im setup.exe 2>nul
taskkill /f /im kartavirya.exe 2>nul

if exist "Release" rmdir /s /q "Release"
mkdir "Release"

if exist "src-tauri\target\release\bundle\nsis" (
    del /f /q "src-tauri\target\release\bundle\nsis\*.exe" 2>nul
)

echo [2/4] Installing dependencies and building Kartavirya (Release Mode)...
call pnpm install
if %ERRORLEVEL% NEQ 0 (
    echo [ERROR] pnpm install failed with error code %ERRORLEVEL%.
    pause
    exit /b %ERRORLEVEL%
)
call pnpm tauri build --bundles nsis
if %ERRORLEVEL% NEQ 0 (
    echo [ERROR] Tauri build failed with error code %ERRORLEVEL%.
    pause
    exit /b %ERRORLEVEL%
)

echo [3/4] Packaging outputs into Release folder...

REM 1. Copy portable standalone executable
if exist "src-tauri\target\release\kartavirya.exe" (
    copy /y "src-tauri\target\release\kartavirya.exe" "Release\portable.exe" >nul
    echo   - Created: Release\portable.exe
) else (
    echo [ERROR] Could not find src-tauri\target\release\kartavirya.exe
    pause
    exit /b 1
)

REM 2. Copy NSIS setup installer (newest built installer)
set SETUP_FOUND=0
for /f "delims=" %%f in ('dir /b /a-d /o-d "src-tauri\target\release\bundle\nsis\*.exe" 2^>nul') do (
    copy /y "src-tauri\target\release\bundle\nsis\%%f" "Release\setup.exe" >nul
    echo   - Created: Release\setup.exe (from %%f)
    set SETUP_FOUND=1
    goto :done_setup
)
:done_setup

if "%SETUP_FOUND%"=="0" (
    for /r "src-tauri\target\release\bundle" %%f in (*setup*.exe *installer*.exe) do (
        copy /y "%%f" "Release\setup.exe" >nul
        echo   - Created: Release\setup.exe
        set SETUP_FOUND=1
        goto :found_setup_fallback
    )
    :found_setup_fallback
    if "%SETUP_FOUND%"=="0" (
        echo [ERROR] Could not locate NSIS setup executable.
        pause
        exit /b 1
    )
)

echo [4/4] Verifying Release folder contents...
dir /b "Release"

echo.
echo ========================================================
echo   Build Successful!
echo   Release deliverables located in: %PROJECT_DIR%Release\
echo     1. setup.exe    (Installer with Desktop Shortcut)
echo     2. portable.exe (Standalone Portable Version)
echo ========================================================
echo.
