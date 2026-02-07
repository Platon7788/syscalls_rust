@echo off
setlocal

echo ========================================
echo SysWhispers3 Syscalls - C Bindings Build
echo ========================================

:: Create output directory
if not exist "lib" mkdir lib

:: Build static library
echo.
echo [1/2] Building syscalls.lib (static)...
cargo build --release
if errorlevel 1 (
    echo ERROR: Build failed!
    exit /b 1
)
copy /Y "target\release\syscalls.lib" "lib\syscalls.lib" >nul 2>&1
if exist "target\release\libsyscalls.a" (
    copy /Y "target\release\libsyscalls.a" "lib\syscalls.lib" >nul
)
echo       syscalls.lib - OK

:: Build DLL
echo.
echo [2/2] Building syscalls.dll (dynamic)...
if exist "target\release\syscalls.dll" (
    copy /Y "target\release\syscalls.dll" "lib\syscalls.dll" >nul
    echo       syscalls.dll - OK
) else (
    echo       syscalls.dll - skipped (staticlib only)
)

:: Show results
echo.
echo ========================================
echo Build complete!
echo ========================================
echo.
echo Output files:
for %%f in (lib\*.*) do (
    echo   %%f
)
echo.
echo Header:
echo   include/syscalls.h
echo.
echo Usage (MSVC):
echo   #include "syscalls.h"
echo   #pragma comment(lib, "syscalls.lib")
echo.

endlocal
