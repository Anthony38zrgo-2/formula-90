@echo off
REM validate.bat — Atajo para ejecutar el validador desde Windows Explorer
REM Uso: arrastra un .obj sobre este .bat, o ejecútalo desde la terminal
REM      validate.bat modelo.obj
REM      validate.bat D:\Modelos\coche

IF "%~1"=="" (
    echo Uso: validate.bat ^<archivo.obj o carpeta^>
    echo Ejemplo: validate.bat ..\current\modelo.obj
    pause
    exit /b 1
)

python "%~dp0validate_obj.py" %*
pause
