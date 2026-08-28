@echo off
setlocal enabledelayedexpansion

REM Package Mirage for Windows (MSI).
REM Run this from a Windows machine with Rust, JDK 21 and Gradle available.

set SCRIPT_DIR=%~dp0
set PROJECT_ROOT=%SCRIPT_DIR%..\..
set DAEMON_DIR=%PROJECT_ROOT%\src\daemon_next
set KMP_DIR=%PROJECT_ROOT%\src\client-kmp
set RESOURCES_DIR=%KMP_DIR%\package-resources

if not exist "%RESOURCES_DIR%\windows" mkdir "%RESOURCES_DIR%\windows"

echo [Windows] Building Rust daemon and CLI...
cd /d "%DAEMON_DIR%"
cargo build --release --bin mirage-daemon --bin mirage
if errorlevel 1 exit /b 1

echo [Windows] Staging binaries for Compose packaging...
copy /Y "%DAEMON_DIR%\target\release\mirage-daemon.exe" "%RESOURCES_DIR%\windows\mirage-daemon.exe" >nul
copy /Y "%DAEMON_DIR%\target\release\mirage.exe" "%RESOURCES_DIR%\windows\mirage.exe" >nul

echo [Windows] Packaging MSI via Gradle...
cd /d "%KMP_DIR%"
call gradlew.bat packageMsi
if errorlevel 1 exit /b 1

echo [Windows] MSI installer:
dir /b "%KMP_DIR%\build\compose\binaries\main-release\msi"\*.msi
