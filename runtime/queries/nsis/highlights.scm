; ── Block Keywords ──

(function_definition "Function" @keyword.function)
(function_definition "FunctionEnd" @keyword.function)

(section_definition "Section" @keyword.storage.type)
(section_definition "SectionEnd" @keyword.storage.type)

(section_group "SectionGroup" @keyword.storage.type)
(section_group "SectionGroupEnd" @keyword.storage.type)

(page_ex_block "PageEx" @keyword.storage.type)
(page_ex_block "PageExEnd" @keyword.storage.type)

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

; ── Macro Invocations ──

; Built-in macros → @function.builtin
(macro_invocation
  name: (define_reference) @function.builtin
  (#match? @function.builtin "^\\$\\{(?i)(If|IfNot|Unless|ElseIf|ElseIfNot|ElseUnless|Else|EndIf|EndUnless|AndIf|AndIfNot|AndUnless|OrIf|OrIfNot|OrUnless|IfCmd|IfThen|IfNotThen|Switch|Select|Case|Case2|Case3|Case4|Case5|CaseElse|Case_Else|Default|EndSwitch|EndSelect|For|ForEach|Next|ExitFor|Do|DoWhile|DoUntil|Loop|LoopWhile|LoopUntil|ExitDo|While|EndWhile|ExitWhile|Break|Continue|Cmd|Abort|Errors|FileExists|RebootFlag|Silent|AltRegView|RtlLanguage|ShellVarContextAll|RegKeyIsEmpty|SectionIsBold|SectionIsExpanded|SectionIsPartiallySelected|SectionIsReadOnly|SectionIsSectionGroup|SectionIsSectionGroupEnd|SectionIsSelected|SectionIsSubSection|SectionIsSubSectionEnd|Contains|ContainsS|EndsWith|EndsWithS|StartsWith|StartsWithS|IsLowerCase|IsUpperCase|IsDomainController|IsNT|IsSafeBootMode|IsServerOS|IsServicePack|IsStarterEdition|IsWin2003R2|OSHasMediaCenter|OSHasTabletSupport|BannerTrimPath|DirState|DriveSpace|GetBaseName|GetDrives|GetExeName|GetExePath|GetFileAttributes|GetFileExt|GetFileName|GetFileVersion|GetOptions|GetOptionsS|GetParameters|GetParent|GetRoot|GetSize|GetTime|Locate|RefreshShellIcons|StrFilter|StrFilterS|VersionCompare|VersionConvert|WordAdd|WordAddS|WordFind|WordFind2X|WordFind2XS|WordFind3X|WordFind3XS|WordFindS|WordInsert|WordInsertS|WordReplace|WordReplaceS|ConfigRead|ConfigReadS|ConfigWrite|ConfigWriteS|FileJoin|FileReadFromEnd|FileRecode|LineFind|LineRead|LineSum|TextCompare|TextCompareS|TrimNewLines|DisableX64FSRedirection|EnableX64FSRedirection|GetNativeMachineArchitecture|IsNativeAMD64|IsNativeARM64|IsNativeIA32|IsNativeMachineArchitecture|IsWow64|RunningX64|AtLeastBuild|AtLeastServicePack|AtLeastWaaS|AtMostBuild|AtMostServicePack|AtMostWaaS|WinVerGetBuild|WinVerGetMajor|WinVerGetMinor|WinVerGetServicePackLevel|MementoSection|MementoSectionDone|MementoSectionEnd|MementoSectionEx|MementoSectionRestore|MementoSectionSave|MementoUnselectedSection)\\}$"))

; User-defined/third-party macros → @function.macro
(macro_invocation
  name: (define_reference) @function.macro)

; ── Labels ──

(label
  name: (identifier) @label)

(label_reference) @label

; ── Constants ──

(identifier) @attribute
(#match? @attribute "^(?i)(ARCHIVE|FILE_ATTRIBUTE_ARCHIVE|FILE_ATTRIBUTE_HIDDEN|FILE_ATTRIBUTE_NORMAL|FILE_ATTRIBUTE_OFFLINE|FILE_ATTRIBUTE_READONLY|FILE_ATTRIBUTE_SYSTEM|FILE_ATTRIBUTE_TEMPORARY|HIDDEN|HKCC|HKCR|HKCR32|HKCR64|HKCU|HKCU32|HKCU64|HKDD|HKEY_CLASSES_ROOT|HKEY_CURRENT_CONFIG|HKEY_CURRENT_USER|HKEY_DYN_DATA|HKEY_LOCAL_MACHINE|HKEY_PERFORMANCE_DATA|HKEY_USERS|HKLM|HKLM32|HKLM64|HKPD|HKU|IDABORT|IDCANCEL|IDD_DIR|IDD_INST|IDD_INSTFILES|IDD_LICENSE|IDD_SELCOM|IDD_UNINST|IDD_VERIFY|IDIGNORE|IDNO|IDOK|IDRETRY|IDYES|MB_ABORTRETRYIGNORE|MB_DEFBUTTON1|MB_DEFBUTTON2|MB_DEFBUTTON3|MB_DEFBUTTON4|MB_ICONEXCLAMATION|MB_ICONINFORMATION|MB_ICONQUESTION|MB_ICONSTOP|MB_OK|MB_OKCANCEL|MB_RETRYCANCEL|MB_RIGHT|MB_RTLREADING|MB_SETFOREGROUND|MB_TOPMOST|MB_USERICON|MB_YESNO|MB_YESNOCANCEL|NORMAL|OFFLINE|READONLY|SHCTX|SHELL_CONTEXT|SW_HIDE|SW_SHOW|SW_SHOWDEFAULT|SW_SHOWMAXIMIZED|SW_SHOWMINIMIZED|SW_SHOWNORMAL|SYSTEM|TEMPORARY)$")

; ── Booleans ──

(identifier) @constant.builtin.boolean
(#match? @constant.builtin.boolean "^(?i)(true|on|false|off)$")

; ── Commands ──

(command
  name: (identifier) @keyword
  (#match? @keyword "^(?i)(Abort|AddBrandingImage|AddSize|AllowRootDirInstall|AllowSkipFiles|AutoCloseWindow|BGFont|BGGradient|BrandingText|BringToFront|Call|CallInstDLL|Caption|ChangeUI|CheckBitmap|ClearErrors|CompletedText|ComponentText|CopyFiles|CPU|CRCCheck|CreateDirectory|CreateFont|CreateShortCut|Delete|DeleteINISec|DeleteINIStr|DeleteRegKey|DeleteRegValue|DetailPrint|DetailsButtonText|DirText|DirVar|DirVerify|EnableWindow|EnumRegKey|EnumRegValue|Exch|Exec|ExecShell|ExecShellWait|ExecWait|ExpandEnvStrings|File|FileBufSize|FileClose|FileErrorText|FileOpen|FileRead|FileReadByte|FileReadUTF16LE|FileReadWord|FileWriteUTF16LE|FileSeek|FileWrite|FileWriteByte|FileWriteWord|FindClose|FindFirst|FindNext|FindWindow|FlushINI|GetCurInstType|GetCurrentAddress|GetDlgItem|GetDLLVersion|GetDLLVersionLocal|GetErrorLevel|GetFileTime|GetFileTimeLocal|GetFullPathName|GetFunctionAddress|GetInstDirError|GetKnownFolderPath|GetLabelAddress|GetRegView|GetShellVarContext|GetTempFileName|GetWinVer|Goto|HideWindow|Icon|IfAbort|IfAltRegView|IfErrors|IfFileExists|IfRebootFlag|IfRtlLanguage|IfShellVarContextAll|IfSilent|InitPluginsDir|InstallButtonText|InstallColors|InstallDir|InstallDirRegKey|InstProgressFlags|InstType|InstTypeGetText|InstTypeSetText|Int64Cmp|Int64CmpU|Int64Fmt|IntCmp|IntCmpU|IntFmt|IntOp|IntPtrCmp|IntPtrCmpU|IntPtrOp|IsWindow|LangString|LicenseBkColor|LicenseData|LicenseForceSelection|LicenseLangString|LicenseText|LoadAndSetImage|LoadLanguageFile|LockWindow|LogSet|LogText|ManifestAppendCustomString|ManifestDisableWindowFiltering|ManifestDPIAware|ManifestDPIAwareness|ManifestGdiScaling|ManifestLongPathAware|ManifestMaxVersionTested|ManifestSupportedOS|MessageBox|MiscButtonText|Name|Nop|OutFile|Page|PageCallbacks|PEAddResource|PEDllCharacteristics|PERemoveResource|PESubsysVer|Pop|Push|Quit|ReadEnvStr|ReadINIStr|ReadMemory|ReadRegDWORD|ReadRegStr|Reboot|RegDLL|Rename|RequestExecutionLevel|ReserveFile|Return|RMDir|SearchPath|SectionGetFlags|SectionGetInstTypes|SectionGetSize|SectionGetText|SectionIn|SectionInstType|SectionSetFlags|SectionSetInstTypes|SectionSetSize|SectionSetText|SendMessage|SetAutoClose|SetBrandingImage|SetCompress|SetCompressionLevel|SetCompressor|SetCompressorDictSize|SetCtlColors|SetCurInstType|SetDatablockOptimize|SetDateSave|SetDetailsPrint|SetDetailsView|SetErrorLevel|SetErrors|SetFileAttributes|SetFont|SetOutPath|SetOverwrite|SetRebootFlag|SetRegView|SetShellVarContext|SetSilent|ShowInstDetails|ShowUninstDetails|ShowWindow|SilentInstall|SilentUnInstall|Sleep|SpaceTexts|StrCmp|StrCmpS|StrCpy|StrLen|SubCaption|Target|Unicode|UninstallButtonText|UninstallCaption|UninstallIcon|UninstallSubCaption|UninstallText|UninstPage|UnRegDLL|UnsafeStrCpy|VIAddVersionKey|VIFileVersion|VIProductVersion|WindowIcon|WriteINIStr|WriteRegBin|WriteRegDWORD|WriteRegExpandStr|WriteRegMultiStr|WriteRegNone|WriteRegStr|WriteUninstaller|XPStyle)$"))
; ── Deprecated Commands ──

(command
  name: (identifier) @keyword.deprecated
  (#match? @keyword.deprecated "^(?i)(CompareDLLVersions|CompareFileTimes|DirShow|DisabledBitmap|EnabledBitmap|GetFullDLLPath|GetParent|GetWinampInstPath|LangStringUP|PackEXEHeader|SectionDivider|SetPluginUnload|SubSection|SubSectionEnd|UninstallExeName)$"))

; ── Variables & References ──

(variable) @variable
(define_reference) @constant.builtin
(lang_string_reference) @string.special

; ── Built-in Variables ──

(variable) @variable.builtin
(#match? @variable.builtin "^\\$(?i)(ADMINTOOLS|APPDATA|CDBURN_AREA|CMDLINE|COMMONFILES|COOKIES|DESKTOP|DOCUMENTS|EXEDIR|EXEFILE|EXEPATH|FAVORITES|FONTS|HISTORY|HWNDPARENT|INSTDIR|INTERNET_CACHE|LANGUAGE|LOCALAPPDATA|MUSIC|NETHOOD|NSIS_MAX_STRLEN|NSIS_VERSION|NSISDIR|OUTDIR|PICTURES|PLUGINSDIR|PRINTHOOD|PROFILE|PROGRAMFILES|PROGRAMFILES32|PROGRAMFILES64|QUICKLAUNCH|RECENT|RESOURCES|RESOURCES_LOCALIZED|SENDTO|SMPROGRAMS|SMSTARTUP|STARTMENU|SYSDIR|TEMP|TEMPLATES|VIDEOS|WINDIR)$")

; ── Strings ──

(string) @string
(raw_string) @string
(backtick_string) @string
(escape_sequence) @constant.character.escape

; ── Numbers ──

(number) @constant.numeric

; ── Flags ──

(flag) @attribute

; ── Operators ──

(comparison_operator) @operator
(pipe_operator) @operator

; ── Comments ──

(comment) @comment.line
(block_comment) @comment.block
