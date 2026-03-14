@echo off
call "C:\Program Files\Microsoft Visual Studio\2022\Professional\VC\Auxiliary\Build\vcvarsall.bat" x86 > nul 2>&1
cd /d D:\GitHub\Rust_Projects\syscalls-rust\c-bindings
cl /nologo /W3 /O2 examples\test_wow64.c /Iinclude /Fe:examples\test_wow64.exe /Fo:examples\test_wow64.obj /link target\i686-pc-windows-msvc\release\syscalls.lib ntdll.lib kernel32.lib > examples\compile_log.txt 2>&1
echo CL_EXIT=%ERRORLEVEL% >> examples\compile_log.txt
if %ERRORLEVEL% EQU 0 (
    examples\test_wow64.exe >> examples\compile_log.txt 2>&1
    echo RUN_EXIT=%ERRORLEVEL% >> examples\compile_log.txt
)
