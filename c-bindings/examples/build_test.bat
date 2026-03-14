@echo off
set "OUTFILE=D:\GitHub\Rust_Projects\syscalls-rust\c-bindings\examples\build_output.txt"
set "SRCDIR=D:\GitHub\Rust_Projects\syscalls-rust\c-bindings\examples"
set "INCDIR=D:\GitHub\Rust_Projects\syscalls-rust\c-bindings\include"
set "LIBFILE=D:\GitHub\Rust_Projects\syscalls-rust\c-bindings\target\i686-pc-windows-msvc\release\syscalls.lib"

echo STARTING > "%OUTFILE%"

call "C:\Program Files\Microsoft Visual Studio\2022\Professional\VC\Auxiliary\Build\vcvarsall.bat" x86 >> "%OUTFILE%" 2>&1

echo VCVARS_DONE >> "%OUTFILE%"

cl /nologo /W3 /O2 /EHa "%SRCDIR%\test_wow64.c" /I"%INCDIR%" /Fe"%SRCDIR%\test_wow64.exe" /Fo"%SRCDIR%\test_wow64.obj" /link "%LIBFILE%" >> "%OUTFILE%" 2>&1

echo CL_EXIT=%ERRORLEVEL% >> "%OUTFILE%"

"%SRCDIR%\test_wow64.exe" >> "%OUTFILE%" 2>&1

echo RUN_EXIT=%ERRORLEVEL% >> "%OUTFILE%"
