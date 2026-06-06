; ── Block Keywords ──

(function_definition "Function" @keyword.function)
(function_definition "FunctionEnd" @keyword.function)

(section_definition "Section" @keyword)
(section_definition "SectionEnd" @keyword)

(section_group "SectionGroup" @keyword)
(section_group "SectionGroupEnd" @keyword)

(page_ex_block "PageEx" @keyword)
(page_ex_block "PageExEnd" @keyword)

(macro_definition "!macro" @keyword.directive)
(macro_definition "!macroend" @keyword.directive)

; ── Block Names ──

(function_definition
  name: (_) @function)

(section_definition
  parameter: (_) @string.special)

(macro_definition
  name: (identifier) @function.macro)

(macro_definition
  parameter: (identifier) @variable.parameter)

; ── Preprocessor ──

(preproc_conditional
  keyword: (preproc_keyword) @keyword.directive)

(preproc_conditional "!endif" @keyword.directive)

(preproc_else "!else" @keyword.directive)

(preproc_directive
  directive: (preproc_keyword) @keyword.directive)

; ── Variable Declaration ──

(variable_declaration "Var" @keyword)
(variable_declaration
  name: (identifier) @variable)

; ── Plugin Calls ──

(plugin_call
  plugin: (identifier) @module)
(plugin_call
  "::" @punctuation.delimiter)
(plugin_call
  function: (identifier) @function.method)

; ── Labels ──

(label
  name: (identifier) @label)

(label_reference) @label

; ── Constants ──

(identifier) @constant
(#match? @constant "^(?i)(ARCHIVE|FILE_ATTRIBUTE_ARCHIVE|FILE_ATTRIBUTE_HIDDEN|FILE_ATTRIBUTE_NORMAL|FILE_ATTRIBUTE_OFFLINE|FILE_ATTRIBUTE_READONLY|FILE_ATTRIBUTE_SYSTEM|FILE_ATTRIBUTE_TEMPORARY|HIDDEN|HKCC|HKCR|HKCR32|HKCR64|HKCU|HKCU32|HKCU64|HKDD|HKEY_CLASSES_ROOT|HKEY_CURRENT_CONFIG|HKEY_CURRENT_USER|HKEY_DYN_DATA|HKEY_LOCAL_MACHINE|HKEY_PERFORMANCE_DATA|HKEY_USERS|HKLM|HKLM32|HKLM64|HKPD|HKU|IDABORT|IDCANCEL|IDD_DIR|IDD_INST|IDD_INSTFILES|IDD_LICENSE|IDD_SELCOM|IDD_UNINST|IDD_VERIFY|IDIGNORE|IDNO|IDOK|IDRETRY|IDYES|MB_ABORTRETRYIGNORE|MB_DEFBUTTON1|MB_DEFBUTTON2|MB_DEFBUTTON3|MB_DEFBUTTON4|MB_ICONEXCLAMATION|MB_ICONINFORMATION|MB_ICONQUESTION|MB_ICONSTOP|MB_OK|MB_OKCANCEL|MB_RETRYCANCEL|MB_RIGHT|MB_RTLREADING|MB_SETFOREGROUND|MB_TOPMOST|MB_USERICON|MB_YESNO|MB_YESNOCANCEL|NORMAL|OFFLINE|READONLY|SHCTX|SHELL_CONTEXT|SW_HIDE|SW_SHOWDEFAULT|SW_SHOWMAXIMIZED|SW_SHOWMINIMIZED|SW_SHOWNORMAL|SYSTEM|TEMPORARY)$")

; ── Booleans ──

(identifier) @boolean
(#match? @boolean "^(?i)(true|on|false|off)$")

; ── Commands ──

(command
  name: (identifier) @keyword
  (#match? @keyword "^(?i)(Abort|AddBrandingImage|AddSize|AllowRootDirInstall|AllowSkipFiles|AutoCloseWindow|BGFont|BGGradient|BrandingText|BringToFront|Call|CallInstDLL|Caption|ChangeUI|CheckBitmap|ClearErrors|CompletedText|ComponentText|CopyFiles|CPU|CRCCheck|CreateDirectory|CreateFont|CreateShortCut|Delete|DeleteINISec|DeleteINIStr|DeleteRegKey|DeleteRegValue|DetailPrint|DetailsButtonText|DirText|DirVar|DirVerify|EnableWindow|EnumRegKey|EnumRegValue|Exch|Exec|ExecShell|ExecShellWait|ExecWait|ExpandEnvStrings|File|FileBufSize|FileClose|FileErrorText|FileOpen|FileRead|FileReadByte|FileReadUTF16LE|FileReadWord|FileWriteUTF16LE|FileSeek|FileWrite|FileWriteByte|FileWriteWord|FindClose|FindFirst|FindNext|FindWindow|FlushINI|GetCurInstType|GetCurrentAddress|GetDlgItem|GetDLLVersion|GetDLLVersionLocal|GetErrorLevel|GetFileTime|GetFileTimeLocal|GetFullPathName|GetFunctionAddress|GetInstDirError|GetKnownFolderPath|GetLabelAddress|GetRegView|GetShellVarContext|GetTempFileName|GetWinVer|Goto|HideWindow|Icon|IfAbort|IfAltRegView|IfErrors|IfFileExists|IfRebootFlag|IfRtlLanguage|IfShellVarContextAll|IfSilent|InitPluginsDir|InstallButtonText|InstallColors|InstallDir|InstallDirRegKey|InstProgressFlags|InstType|InstTypeGetText|InstTypeSetText|Int64Cmp|Int64CmpU|Int64Fmt|IntCmp|IntCmpU|IntFmt|IntOp|IntPtrCmp|IntPtrCmpU|IntPtrOp|IsWindow|LangString|LicenseBkColor|LicenseData|LicenseForceSelection|LicenseLangString|LicenseText|LoadAndSetImage|LoadLanguageFile|LockWindow|LogSet|LogText|ManifestAppendCustomString|ManifestDisableWindowFiltering|ManifestDPIAware|ManifestGdiScaling|ManifestLongPathAware|ManifestMaxVersionTested|ManifestSupportedOS|MessageBox|MiscButtonText|Name|Nop|OutFile|Page|PageCallbacks|PEAddResource|PEDllCharacteristics|PERemoveResource|PESubsysVer|Pop|Push|Quit|ReadEnvStr|ReadINIStr|ReadMemory|ReadRegDWORD|ReadRegStr|Reboot|RegDLL|Rename|RequestExecutionLevel|ReserveFile|Return|RMDir|SearchPath|SectionGetFlags|SectionGetInstTypes|SectionGetSize|SectionGetText|SectionIn|SectionSetFlags|SectionSetInstTypes|SectionSetSize|SectionSetText|SendMessage|SetAutoClose|SetBrandingImage|SetCompress|SetCompressionLevel|SetCompressor|SetCompressorDictSize|SetCtlColors|SetCurInstType|SetDatablockOptimize|SetDateSave|SetDetailsPrint|SetDetailsView|SetErrorLevel|SetErrors|SetFileAttributes|SetFont|SetOutPath|SetOverwrite|SetRebootFlag|SetRegView|SetShellVarContext|SetSilent|ShowInstDetails|ShowUninstDetails|ShowWindow|SilentInstall|SilentUnInstall|Sleep|SpaceTexts|StrCmp|StrCmpS|StrCpy|StrLen|SubCaption|Target|Unicode|UninstallButtonText|UninstallCaption|UninstallIcon|UninstallSubCaption|UninstallText|UninstPage|UnRegDLL|UnsafeStrCpy|VIAddVersionKey|VIFileVersion|VIProductVersion|WindowIcon|WriteINIStr|WriteRegBin|WriteRegDWORD|WriteRegExpandStr|WriteRegMultiStr|WriteRegNone|WriteRegStr|WriteUninstaller|XPStyle)$"))

; ── Deprecated Commands ──

(command
  name: (identifier) @warning
  (#match? @warning "^(?i)(CompareDLLVersions|CompareFileTimes|DirShow|DisabledBitmap|EnabledBitmap|GetFullDLLPath|GetParent|GetWinampInstPath|LangStringUP|PackEXEHeader|SectionDivider|SetPluginUnload|SubSection|SubSectionEnd|UninstallExeName)$"))

; ── Variables & References ──

(variable) @variable
(define_reference) @constant.macro
(lang_string_reference) @string.special

; ── Built-in Variables ──

(variable) @variable.builtin
(#match? @variable.builtin "^\\$(?i)(ADMINTOOLS|APPDATA|CDBURN_AREA|CMDLINE|COMMONFILES|COOKIES|DESKTOP|DOCUMENTS|EXEDIR|EXEFILE|EXEPATH|FAVORITES|FONTS|HISTORY|HWNDPARENT|INSTDIR|INTERNET_CACHE|LANGUAGE|LOCALAPPDATA|MUSIC|NETHOOD|NSIS_MAX_STRLEN|NSIS_VERSION|NSISDIR|OUTDIR|PICTURES|PLUGINSDIR|PRINTHOOD|PROFILE|PROGRAMFILES|PROGRAMFILES32|PROGRAMFILES64|QUICKLAUNCH|RECENT|RESOURCES|RESOURCES_LOCALIZED|SENDTO|SMPROGRAMS|SMSTARTUP|STARTMENU|SYSDIR|TEMP|TEMPLATES|VIDEOS|WINDIR)$")

; ── Strings ──

(string) @string
(raw_string) @string
(backtick_string) @string
(escape_sequence) @string.escape

; ── Numbers ──

(number) @number

; ── Flags ──

(flag) @attribute

; ── Operators ──

(comparison_operator) @operator
(pipe_operator) @operator

; ── Comments ──

(comment) @comment.line
(block_comment) @comment.block
