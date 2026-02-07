@echo off
REM Build script for syscalls C bindings
REM Builds both MSVC and MinGW versions

echo ========================================
echo Building syscalls C bindings
echo ========================================

REM Create output directory
if not exist "lib" mkdir lib

echo.
echo [1/2] Building MSVC x64...
cargo build --release --target x86_64-pc-windows-msvc
if errorlevel 1 goto :error

echo.
echo [2/2] Building MinGW x64 (for RAD Studio)...
cargo build --release --target x86_64-pc-windows-gnu
if errorlevel 1 goto :error

echo.
echo Copying libraries to lib/...

REM MSVC
copy /Y "target\x86_64-pc-windows-msvc\release\syscalls.lib" "lib\syscalls_msvc.lib"
copy /Y "target\x86_64-pc-windows-msvc\release\syscalls.dll" "lib\syscalls.dll"

REM MinGW (for RAD Studio / Delphi / C++ Builder)
copy /Y "target\x86_64-pc-windows-gnu\release\libsyscalls.a" "lib\syscalls_mingw.a"

echo.
echo ========================================
echo Build complete!
echo ========================================
echo.
echo Output files in lib/:
echo   syscalls_msvc.lib  - MSVC static library
echo   syscalls_mingw.a   - MinGW static library (RAD Studio)
echo   syscalls.dll       - Dynamic library
echo.
echo Header: include/syscalls.h
echo.
goto :end

:error
echo.
echo BUILD FAILED!
exit /b 1

:end
