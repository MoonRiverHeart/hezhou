@echo off
@rem Hvigor wrapper for Harmony project
@rem Usage: hvigorw.bat [task]

setlocal enabledelayedexpansion

set PROJECT_DIR=%~dp0
set TOOLS_DIR=C:\Users\94023\Documents\commandline-tools-windows-x64\command-line-tools
set NODE_EXE=%TOOLS_DIR%\tool\node\node.exe
set HVIGORW_JS=%TOOLS_DIR%\hvigor\bin\hvigorw.js

@rem Add Node to PATH
set PATH=%TOOLS_DIR%\tool\node;%PATH%

@rem Add Java to PATH if JAVA_HOME is set
if defined JAVA_HOME (
    set PATH=%JAVA_HOME%\bin;%PATH%
) else (
    where java >nul 2>&1
    if errorlevel 1 (
        echo WARNING: Java not found in PATH. HAP packaging requires Java.
        echo Please install JDK or set JAVA_HOME environment variable.
    )
)

if not exist "%NODE_EXE%" (
    echo ERROR: Node.js not found at %NODE_EXE%
    exit /b 1
)

if not exist "%HVIGORW_JS%" (
    echo ERROR: hvigorw.js not found at %HVIGORW_JS%
    exit /b 1
)

pushd "%PROJECT_DIR%"
"%NODE_EXE%" "%HVIGORW_JS%" %*
popd
