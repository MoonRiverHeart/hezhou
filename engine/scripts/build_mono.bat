@echo off
REM Build Mono version of C# scripts
REM Requires Mono SDK installed (mcs compiler)

set SCRIPTS_DIR=%~dp0
set OUTPUT_DIR=%SCRIPTS_DIR%bin\Mono\Release\net8.0

REM Create output directory if it doesn't exist
if not exist "%OUTPUT_DIR%" mkdir "%OUTPUT_DIR%"

REM Compile with Mono compiler
"C:\Program Files\Mono\bin\mcs.bat" -target:library -out:"%OUTPUT_DIR%\RotationScript.Mono.dll" RotationScript.cs -define:MONO

if %ERRORLEVEL% equ 0 (
    echo [Success] RotationScript.Mono.dll compiled
) else (
    echo [Error] Compilation failed
    exit /b 1
)