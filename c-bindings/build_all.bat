@echo off
setlocal

echo ============================================================
echo  SysWhispers3 Syscalls - C Bindings Build (6 variants)
echo ============================================================
echo.
echo  MSVC x86:  debug + release  (static CRT)
echo  MSVC x64:  debug + release  (static CRT)
echo  MinGW x64: debug + release
echo.

REM Create output directory structure
if not exist "lib" mkdir lib
if not exist "lib\msvc_x86\debug" mkdir lib\msvc_x86\debug
if not exist "lib\msvc_x86\release" mkdir lib\msvc_x86\release
if not exist "lib\msvc_x64\debug" mkdir lib\msvc_x64\debug
if not exist "lib\msvc_x64\release" mkdir lib\msvc_x64\release
if not exist "lib\mingw_x64\debug" mkdir lib\mingw_x64\debug
if not exist "lib\mingw_x64\release" mkdir lib\mingw_x64\release

set ERRORS=0
set OK_COUNT=0
set TOTAL=6

REM Static CRT for MSVC targets
set MSVC_RUSTFLAGS=-C target-feature=+crt-static

REM ============================================================
REM  [1/6] MSVC x86 Debug
REM ============================================================
echo [1/%TOTAL%] Building MSVC x86 Debug (i686-pc-windows-msvc)...

rustup target list --installed 2>nul | findstr /C:"i686-pc-windows-msvc" >nul 2>&1
if errorlevel 1 (
    echo       Installing target i686-pc-windows-msvc...
    rustup target add i686-pc-windows-msvc
)

set RUSTFLAGS=%MSVC_RUSTFLAGS%
cargo build --target i686-pc-windows-msvc 2>&1
if errorlevel 1 (
    echo       [FAIL] MSVC x86 Debug
    set /a ERRORS+=1
    goto :step2
)
copy /Y "target\i686-pc-windows-msvc\debug\syscalls.lib" "lib\msvc_x86\debug\syscalls.lib" >nul 2>&1
if exist "target\i686-pc-windows-msvc\debug\syscalls.dll" (
    copy /Y "target\i686-pc-windows-msvc\debug\syscalls.dll" "lib\msvc_x86\debug\syscalls.dll" >nul 2>&1
    copy /Y "target\i686-pc-windows-msvc\debug\syscalls.dll.lib" "lib\msvc_x86\debug\syscalls.dll.lib" >nul 2>&1
)
echo       [OK] MSVC x86 Debug
set /a OK_COUNT+=1

:step2
echo.

REM ============================================================
REM  [2/6] MSVC x86 Release
REM ============================================================
echo [2/%TOTAL%] Building MSVC x86 Release (i686-pc-windows-msvc)...

set RUSTFLAGS=%MSVC_RUSTFLAGS%
cargo build --release --target i686-pc-windows-msvc 2>&1
if errorlevel 1 (
    echo       [FAIL] MSVC x86 Release
    set /a ERRORS+=1
    goto :step3
)
copy /Y "target\i686-pc-windows-msvc\release\syscalls.lib" "lib\msvc_x86\release\syscalls.lib" >nul 2>&1
if exist "target\i686-pc-windows-msvc\release\syscalls.dll" (
    copy /Y "target\i686-pc-windows-msvc\release\syscalls.dll" "lib\msvc_x86\release\syscalls.dll" >nul 2>&1
    copy /Y "target\i686-pc-windows-msvc\release\syscalls.dll.lib" "lib\msvc_x86\release\syscalls.dll.lib" >nul 2>&1
)
echo       [OK] MSVC x86 Release
set /a OK_COUNT+=1

:step3
echo.

REM ============================================================
REM  [3/6] MSVC x64 Debug
REM ============================================================
echo [3/%TOTAL%] Building MSVC x64 Debug (x86_64-pc-windows-msvc)...

set RUSTFLAGS=%MSVC_RUSTFLAGS%
cargo build --target x86_64-pc-windows-msvc 2>&1
if errorlevel 1 (
    echo       [FAIL] MSVC x64 Debug
    set /a ERRORS+=1
    goto :step4
)
copy /Y "target\x86_64-pc-windows-msvc\debug\syscalls.lib" "lib\msvc_x64\debug\syscalls.lib" >nul 2>&1
if exist "target\x86_64-pc-windows-msvc\debug\syscalls.dll" (
    copy /Y "target\x86_64-pc-windows-msvc\debug\syscalls.dll" "lib\msvc_x64\debug\syscalls.dll" >nul 2>&1
    copy /Y "target\x86_64-pc-windows-msvc\debug\syscalls.dll.lib" "lib\msvc_x64\debug\syscalls.dll.lib" >nul 2>&1
)
echo       [OK] MSVC x64 Debug
set /a OK_COUNT+=1

:step4
echo.

REM ============================================================
REM  [4/6] MSVC x64 Release
REM ============================================================
echo [4/%TOTAL%] Building MSVC x64 Release (x86_64-pc-windows-msvc)...

set RUSTFLAGS=%MSVC_RUSTFLAGS%
cargo build --release --target x86_64-pc-windows-msvc 2>&1
if errorlevel 1 (
    echo       [FAIL] MSVC x64 Release
    set /a ERRORS+=1
    goto :step5
)
copy /Y "target\x86_64-pc-windows-msvc\release\syscalls.lib" "lib\msvc_x64\release\syscalls.lib" >nul 2>&1
if exist "target\x86_64-pc-windows-msvc\release\syscalls.dll" (
    copy /Y "target\x86_64-pc-windows-msvc\release\syscalls.dll" "lib\msvc_x64\release\syscalls.dll" >nul 2>&1
    copy /Y "target\x86_64-pc-windows-msvc\release\syscalls.dll.lib" "lib\msvc_x64\release\syscalls.dll.lib" >nul 2>&1
)
echo       [OK] MSVC x64 Release
set /a OK_COUNT+=1

:step5
echo.

REM Clear RUSTFLAGS for MinGW builds
set RUSTFLAGS=

REM ============================================================
REM  [5/6] MinGW x64 Debug
REM ============================================================
echo [5/%TOTAL%] Building MinGW x64 Debug (x86_64-pc-windows-gnu)...

rustup target list --installed 2>nul | findstr /C:"x86_64-pc-windows-gnu" >nul 2>&1
if errorlevel 1 (
    echo       Installing target x86_64-pc-windows-gnu...
    rustup target add x86_64-pc-windows-gnu
)

cargo build --target x86_64-pc-windows-gnu 2>&1
if errorlevel 1 (
    echo       [FAIL] MinGW x64 Debug
    set /a ERRORS+=1
    goto :step6
)
copy /Y "target\x86_64-pc-windows-gnu\debug\libsyscalls.a" "lib\mingw_x64\debug\libsyscalls.a" >nul 2>&1
if exist "target\x86_64-pc-windows-gnu\debug\syscalls.dll" (
    copy /Y "target\x86_64-pc-windows-gnu\debug\syscalls.dll" "lib\mingw_x64\debug\syscalls.dll" >nul 2>&1
)
echo       [OK] MinGW x64 Debug
set /a OK_COUNT+=1

:step6
echo.

REM ============================================================
REM  [6/6] MinGW x64 Release
REM ============================================================
echo [6/%TOTAL%] Building MinGW x64 Release (x86_64-pc-windows-gnu)...

cargo build --release --target x86_64-pc-windows-gnu 2>&1
if errorlevel 1 (
    echo       [FAIL] MinGW x64 Release
    set /a ERRORS+=1
    goto :results
)
copy /Y "target\x86_64-pc-windows-gnu\release\libsyscalls.a" "lib\mingw_x64\release\libsyscalls.a" >nul 2>&1
if exist "target\x86_64-pc-windows-gnu\release\syscalls.dll" (
    copy /Y "target\x86_64-pc-windows-gnu\release\syscalls.dll" "lib\mingw_x64\release\syscalls.dll" >nul 2>&1
)
echo       [OK] MinGW x64 Release
set /a OK_COUNT+=1

:results
echo.

REM ============================================================
REM  Backward compatibility flat layout
REM ============================================================
echo Copying flat layout (backward compatibility)...

REM MSVC x64 release as default
if exist "lib\msvc_x64\release\syscalls.lib" (
    copy /Y "lib\msvc_x64\release\syscalls.lib" "lib\syscalls_msvc.lib" >nul 2>&1
)
if exist "lib\msvc_x64\release\syscalls.dll" (
    copy /Y "lib\msvc_x64\release\syscalls.dll" "lib\syscalls.dll" >nul 2>&1
)
if exist "lib\mingw_x64\release\libsyscalls.a" (
    copy /Y "lib\mingw_x64\release\libsyscalls.a" "lib\syscalls_mingw.a" >nul 2>&1
)

REM ============================================================
REM  Also copy to target dirs for Detours project compatibility
REM ============================================================
echo Copying to target dirs (Detours project compatibility)...

REM MSVC x86
if exist "lib\msvc_x86\debug\syscalls.lib" (
    if not exist "target\i686-pc-windows-msvc\debug" mkdir target\i686-pc-windows-msvc\debug
    copy /Y "lib\msvc_x86\debug\syscalls.lib" "target\i686-pc-windows-msvc\debug\syscalls.lib" >nul 2>&1
)
if exist "lib\msvc_x86\release\syscalls.lib" (
    if not exist "target\i686-pc-windows-msvc\release" mkdir target\i686-pc-windows-msvc\release
    copy /Y "lib\msvc_x86\release\syscalls.lib" "target\i686-pc-windows-msvc\release\syscalls.lib" >nul 2>&1
)

REM MSVC x64
if exist "lib\msvc_x64\debug\syscalls.lib" (
    if not exist "target\x86_64-pc-windows-msvc\debug" mkdir target\x86_64-pc-windows-msvc\debug
    copy /Y "lib\msvc_x64\debug\syscalls.lib" "target\x86_64-pc-windows-msvc\debug\syscalls.lib" >nul 2>&1
)
if exist "lib\msvc_x64\release\syscalls.lib" (
    if not exist "target\x86_64-pc-windows-msvc\release" mkdir target\x86_64-pc-windows-msvc\release
    copy /Y "lib\msvc_x64\release\syscalls.lib" "target\x86_64-pc-windows-msvc\release\syscalls.lib" >nul 2>&1
)

REM ============================================================
REM  Results
REM ============================================================
echo.
echo ============================================================
echo  BUILD COMPLETE: %OK_COUNT%/%TOTAL% OK, %ERRORS%/%TOTAL% FAILED
echo ============================================================
echo.
echo Output structure:
echo.
echo   lib\
echo   +-- msvc_x86\
echo   ^|   +-- debug\             MSVC x86 Debug (/MTd)
echo   ^|   ^|   +-- syscalls.lib
echo   ^|   +-- release\           MSVC x86 Release (/MT)
echo   ^|       +-- syscalls.lib
echo   +-- msvc_x64\
echo   ^|   +-- debug\             MSVC x64 Debug (/MTd)
echo   ^|   ^|   +-- syscalls.lib
echo   ^|   +-- release\           MSVC x64 Release (/MT)
echo   ^|       +-- syscalls.lib
echo   +-- mingw_x64\
echo   ^|   +-- debug\             MinGW x64 Debug
echo   ^|   ^|   +-- libsyscalls.a
echo   ^|   +-- release\           MinGW x64 Release
echo   ^|       +-- libsyscalls.a
echo   +-- syscalls_msvc.lib      Flat compat (= msvc_x64 release)
echo   +-- syscalls_mingw.a       Flat compat (= mingw_x64 release)
echo.
echo Header: include\syscalls.h
echo.
echo All MSVC builds use static CRT (+crt-static).
echo.

if %ERRORS% GTR 0 exit /b 1
endlocal
