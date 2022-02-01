@echo off

set GRAMMARS_DIR=%~dp0..\helix-syntax\languages
set REVISIONS_FILE=%~dp0\revisions.txt
set REMOTE_NAME=helix-origin

call :do_%1% 2>nul
if errorlevel 1 call :print_usage_and_exit
exit /B

:print_usage_and_exit
  echo Usage: %~f0 ^<command^>
  echo.
  echo Commands:
  echo   status  Checks that each grammar is checked out at the revision in revisions.txt
  echo   sync    Ensures all grammars are cloned at the revisions in revisions.txt
  echo   clean   Removes all grammars from the grammars directory
  echo.
  exit /B 1

:ensure_grammar_fetched
  setlocal
  set "grammar_dir=%GRAMMARS_DIR%\%~nx1"
  set remote_url=%~1
  set revision=%~2

  if not exist %grammar_dir% (
    mkdir %grammar_dir%
  )

  pushd %grammar_dir%
  if not exist .git (
    git init
  )
  git remote set-url %REMOTE_NAME% %remote_url% 2>NUL
  if errorlevel 1 git remote add %REMOTE_NAME% %remote_url%
  git fetch %REMOTE_NAME% %revision% --depth=1
  git checkout %revision%
  popd
  exit /B 0

:check_grammar_status
  setlocal
  set "grammar=%~nx1"
  set "grammar_dir=%GRAMMARS_DIR%\%grammar%"
  set remote_url=%~1
  set expected_revision=%~2

  pushd %grammar_dir%
  for /F "tokens=*" %%r in ('git rev-parse HEAD') do (
    set current_revision=%%r
  )
  popd

  if "%current_revision%" == "%expected_revision%" (
    exit /B 0
  ) else (
    endlocal
    set are_any_out_of_date=true
    echo %grammar% is out of date.
    exit /B 1
  )

:do_clean
  pushd %GRAMMARS_DIR%
  for /D %%d in (tree-sitter-*) do (
    rmdir /s /q %%d
  )
  popd
  exit /B 0

:do_sync
  for /F "tokens=1,2" %%i in (%REVISIONS_FILE%) do (
    call :ensure_grammar_fetched %%i %%j
  )
  exit /B


:do_status
  set are_any_out_of_date=false
  for /F "tokens=1,2" %%i in (%REVISIONS_FILE%) do (
    call :check_grammar_status %%i %%j
  )

  if "%are_any_out_of_date%" == "false" (
    echo All grammars are up to date.
  )
  exit /B
