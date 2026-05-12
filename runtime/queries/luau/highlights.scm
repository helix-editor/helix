"return" @keyword.control.return

"local" @keyword.storage.modifier

"type" @keyword.storage.type

"function" @keyword.function

(break_stmt) @keyword.control
(continue_stmt) @keyword.control
(readwrite) @keyword.storage.modifier

[
  "do"
  "end"
] @keyword

[
  "while"
  "repeat"
  "until"
  "for"
] @keyword.control.repeat

[
  "if"
  "elseif"
  "else"
  "then"
] @keyword.control.conditional

[
  "in"
  "and"
  "or"
  "not"
] @keyword.operator

(ifexp
[
  "if"
  "then"
  "elseif"
  "else"
] @keyword.operator)

(type_stmt "export" @keyword.control.import) 

(_
  operator: [
    "+" "-" "*"  "/"  "//" "%"
    "^" "#" "==" "~=" "<=" ">="
    "<" ">" "&"  "|"  "->" "::"
    ".."
  ] @operator
)

(_
  assign_symbol: [
    "="  "+=" "-=" "*=" "/=" "//="
    "%=" "^=" "..="
  ] @operator
)

[
  ";"
  ":"
  ","
  "."
] @punctuation.delimiter

(string) @string

(_
  variable_name: (name) @variable
)

(_
  parameter_name: (name) @variable.parameter
)

(_
  method_name: (name) @function.method
)

(_
  function_name: (name) @function
)

(_
  table_name: (name) @namespace
)

(_
  field_name: (name) @variable.other.member
)

(table
[
  "{"
  "}"
] @constructor)

; special comment directives
(chunk
  .
  (comment)*
  .
  (comment) @keyword.directive
  (#match? @keyword.directive "^--!(strict|native)[\r]?$")
)

(comment) @comment

(number) @constant.numeric

(unicode_escape
  "{" @punctuation.special
  "codepoint" @constant.numeric.integer
  "}" @punctuation.special
)
(unicode_escape) @constant.character.escape
(dec_byte_escape) @constant.character.escape
(hex_byte_escape) @constant.character.escape
(simple_escape) @constant.character.escape

(interp_start) @punctuation.special
(interp_content) @string
(interp_brace_open) @punctuation.special
(interp_brace_close) @punctuation.special
(interp_end) @punctuation.special

[
 "("
 ")"
 "["
 "]"
 "{"
 "}"
 "<"
 ">"
] @punctuation.bracket

(_
  type_name: (name) @type
)

(type_stmt
  left: (name) @type
)

(_
  attribute_name: (name) @attribute
)

(exp
  (vararg) @constant
)

(nil) @constant.builtin

(boolean) @constant.builtin.boolean

(_
  generic_type_name: (name) @type.parameter
)

(_
  generic_typepack_name: (name) @type.parameter
)

(dyntype
  "typeof" @keyword.directive
)

(_
  module_namespace: (name) @namespace
)

; if the value of the field is a function, then
; color the name of a field assignment as a method
(field
  field_name: (name) @function.method
  value: (anon_fn)
)

; if a call statement is an invocation on a variable,
; color the last name in a name sequence as a function
(call_stmt
  invoked: (var
    variable_name: (name) @function
  )
)

(call_stmt
  invoked: (var
    table_name: (name) @namespace
    (key
      field_name: (name) @function
    )
    .
  )
)

(call_stmt
  invoked: (_
    (key
      field_name: (name) @function
    )
    .
  )
)

(fn_stmt
  (key
    field_name: (name) @function.method
  )
  .
  (paramlist)?
)

(_
  type_name: (name) @type.builtin
  (#any-of? @type.builtin
    "number" "string" "any"
    "never"                             "unknown"                          "boolean"
    "thread"                            "userdata"                         "Accessory"
    "Accoutrement"                      "Actor"                            "AdGui"
    "AdPortal"                          "AdService"                        "AdvancedDragger"
    "AirController"                     "AlignOrientation"                 "AlignPosition"
    "AnalysticsSettings"                "AnalyticsService"                 "AngularVelocity"
    "Animation"                         "AnimationClip"                    "AnimationClipProvider"
    "AnimationController"               "AnimationFromVideoCreatorService" "AnimationFromVideoCreatorStudioService"
    "AnimationRigData"                  "AnimationStreamTrack"             "AnimationTrack"
    "Animator"                          "AppStorageService"                "AppUpdateService"
    "ArcHandles"                        "AssetCounterService"              "AssetDeliveryProxy"
    "AssetImportService"                "AssetImportSession"               "AssetManagerService"
    "AssetService"                      "AssetSoundEffect"                 "Atmosphere"
    "Attachment"                        "AvatarEditorService"              "AvatarImportService"
    "Axes"                              "Backpack"                         "BackpackItem"
    "BadgeService"                      "BallSocketConstraint"             "BasePart"
    "BasePlayerGui"                     "BaseScript"                       "BaseWrap"
    "Beam"                              "BillboardGui"                     "BinaryStringValue"
    "BindableEvent"                     "BindableFunction"                 "BloomEffect"
    "BlurEffect"                        "BodyAngularVelocity"              "BodyColors"
    "BodyForce"                         "BodyGyro"                         "BodyMover"
    "BodyPosition"                      "BodyThrust"                       "BodyVelocity"
    "Bone"                              "BoolValue"                        "BoxHandleAdornment"
    "Breakpoint"                        "BrickColor"                       "BrickColorValue"
    "BrowserService"                    "BubbleChatConfiguration"          "BulkImportService"
    "CFrame"                            "CFrameValue"                      "CSGDictionaryService"
    "CacheableContentProvider"          "CalloutService"                   "Camera"
    "CanvasGroup"                       "CatalogPages"                     "CatalogSearchParams"
    "ChangeHistoryService"              "ChannelSelectorSoundEffect"       "CharacterAppearance"
    "CharacterMesh"                     "Chat"                             "ChatInputBarConfiguration"
    "ChatWindowConfiguration"           "ChorusSoundEffect"                "ClickDetector"
    "ClientReplicator"                  "ClimbController"                  "Clothing"
    "CloudLocalizationTable"            "Clouds"                           "ClusterPacketCache"
    "CollectionService"                 "Color3"                           "ColorCorrectionEffect"
    "ColorSequence"                     "ColorSequenceKeypoint"            "CommandInstance"
    "CommandService"                    "CompressorSoundEffect"            "ConeHandleAdornment"
    "Configuration"                     "ConfigureServerService"           "Constraint"
    "Content"                           "ContentProvider"                  "ContextActionService"
    "Controller"                        "ControllerBase"                   "ControllerManager"
    "ControllerService"                 "CookiesService"                   "CoreGui"
    "CorePackages"                      "CoreScript"                       "CoreScriptSyncService"
    "CornerWedgePart"                   "CrossDMScriptChangeListener"      "CurveAnimation"
    "CustomSoundEffect"                 "CylinderHandleAdornment"          "CylindricalConstraint"
    "DataModel"                         "DataModelMesh"                    "DataModelPatchService"
    "DataModelSession"                  "DataStore"                        "DataStoreIncrementOptions"
    "DataStoreInfo"                     "DataStoreKey"                     "DataStoreKeyInfo"
    "DataStoreKeyPages"                 "DataStoreListingPages"            "DataStoreObjectVersionInfo"
    "DataStoreOptions"                  "DataStorePages"                   "DataStoreService"
    "DataStoreSetOptions"               "DataStoreVersionPages"            "DateTime"
    "Debris"                            "DebugSettings"                    "DebuggablePluginWatcher"
    "DebuggerBreakpoint"                "DebuggerConnection"               "DebuggerConnectionManager"
    "DebuggerLuaResponse"               "DebuggerManager"                  "DebuggerUIService"
    "DebuggerVariable"                  "DebuggerWatch"                    "Decal"
    "DepthOfFieldEffect"                "DeviceIdService"                  "Dialog"
    "DialogChoice"                      "DistortionSoundEffect"            "DockWidgetPluginGui"
    "DockWidgetPluginGuiInfo"           "DraftsService"                    "Dragger"
    "DraggerService"                    "DynamicRotate"                    "EchoSoundEffect"
    "EditableImage"                     "EditableMesh"                     "EmotesPages"
    "Enum"                              "EnumItem"                         "Enums"
    "EqualizerSoundEffect"              "EulerRotationCurve"               "EventIngestService"
    "ExperienceInviteOptions"           "Explosion"                        "FaceAnimatorService"
    "FaceControls"                      "FaceInstance"                     "Faces"
    "FacialAnimationRecordingService"   "FacialAnimationStreamingService"  "Feature"
    "File"                              "FileMesh"                         "Fire"
    "FlagStandService"                  "FlangeSoundEffect"                "FloatCurve"
    "FlyweightService"                  "Folder"                           "Font"
    "ForceField"                        "FormFactorPart"                   "Frame"
    "FriendPages"                       "FriendService"                    "GamePassService"
    "GameSettings"                      "GamepadService"                   "GenericSettings"
    "Geometry"                          "GetTextBoundsParams"              "GlobalDataStore"
    "GlobalSettings"                    "GoogleAnalyticsConfiguration"     "GroundController"
    "GroupService"                      "GuiBase"                          "GuiButton"
    "GuiLabel"                          "GuiObject"                        "GuiService"
    "GuidRegistryService"               "HSRDataContentProvider"           "HandleAdornment"
    "Handles"                           "HandlesBase"                      "HapticService"
    "HeightmapImporterService"          "HiddenSurfaceRemovalAsset"        "Highlight"
    "HingeConstraint"                   "HttpRbxApiService"                "HttpRequest"
    "HttpService"                       "Humanoid"                         "HumanoidController"
    "HumanoidDescription"               "IKControl"                        "ILegacyStudioBridge"
    "IXPService"                        "ImageButton"                      "ImageHandleAdornment"
    "ImageLabel"                        "ImporterAnimationSettings"        "ImporterBaseSettings"
    "ImporterFacsSettings"              "ImporterGroupSettings"            "ImporterJointSettings"
    "ImporterMaterialSettings"          "ImporterMeshSettings"             "ImporterRootSettings"
    "IncrementalPatchBuilder"           "InputObject"                      "InsertService"
    "Instance"                          "InstanceAdornment"                "IntValue"
    "InventoryPages"                    "JointInstance"                    "KeyboardService"
    "Keyframe"                          "KeyframeMarker"                   "KeyframeSequence"
    "KeyframeSequenceProvider"          "LSPFileSyncService"               "LanguageService"
    "LayerCollector"                    "LegacyStudioBridge"               "Light"
    "Lighting"                          "LineForce"                        "LineHandleAdornment"
    "LinearVelocity"                    "LocalDebuggerConnection"          "LocalScript"
    "LocalStorageService"               "LocalizationService"              "LocalizationTable"
    "LodDataEntity"                     "LodDataService"                   "LogService"
    "LoginService"                      "LuaSettings"                      "LuaSourceContainer"
    "LuaWebService"                     "LuauScriptAnalyzerService"        "MarkerCurve"
    "MarketplaceService"                "MaterialService"                  "MaterialVariant"
    "MemStorageConnection"              "MemStorageService"                "MemoryStoreQueue"
    "MemoryStoreService"                "MemoryStoreSortedMap"             "MeshContentProvider"
    "MeshPart"                          "MessageBusConnection"             "MessageBusService"
    "MessagingService"                  "MetaBreakpoint"                   "MetaBreakpointContext"
    "MetaBreakpointManager"             "Model"                            "ModuleScript"
    "Motor"                             "Mouse"                            "MouseService"
    "MultipleDocumentInterfaceInstance" "NegateOperation"                  "NetworkClient"
    "NetworkMarker"                     "NetworkPeer"                      "NetworkReplicator"
    "NetworkServer"                     "NetworkSettings"                  "NoCollisionConstraint"
    "NonReplicatedCSGDictionaryService" "NotificationService"              "NumberPose"
    "NumberRange"                       "NumberSequence"                   "NumberSequenceKeypoint"
    "NumberValue"                       "Object"                           "ObjectValue"
    "OrderedDataStore"                  "OutfitPages"                      "OverlapParams"
    "PVAdornment"                       "PVInstance"                       "PackageLink"
    "PackageService"                    "PackageUIService"                 "Pages"
    "Pants"                             "ParabolaAdornment"                "Part"
    "PartAdornment"                     "PartOperation"                    "PartOperationAsset"
    "ParticleEmitter"                   "PatchMapping"                     "Path"
    "PathWaypoint"                      "PathfindingLink"                  "PathfindingModifier"
    "PathfindingService"                "PausedState"                      "PausedStateBreakpoint"
    "PausedStateException"              "PermissionsService"               "PhysicalProperties"
    "PhysicsService"                    "PhysicsSettings"                  "PitchShiftSoundEffect"
    "PlaneConstraint"                   "Platform"                         "Player"
    "PlayerEmulatorService"             "PlayerGui"                        "PlayerMouse"
    "PlayerScripts"                     "Players"                          "Plugin"
    "PluginAction"                      "PluginDebugService"               "PluginDragEvent"
    "PluginGui"                         "PluginGuiService"                 "PluginManagementService"
    "PluginManager"                     "PluginManagerInterface"           "PluginMenu"
    "PluginMouse"                       "PluginPolicyService"              "PluginToolbar"
    "PluginToolbarButton"               "PointLight"                       "PolicyService"
    "Pose"                              "PoseBase"                         "PostEffect"
    "PrismaticConstraint"               "ProcessInstancePhysicsService"    "ProximityPrompt"
    "ProximityPromptService"            "PublishService"                   "QWidgetPluginGui"
    "RBXScriptConnection"               "Random"                           "Ray"
    "RayValue"                          "RaycastParams"                    "RaycastResult"
    "RbxAnalyticsService"               "Rect"                             "ReflectionMetadata"
    "ReflectionMetadataCallbacks"       "ReflectionMetadataClass"          "ReflectionMetadataClasses"
    "ReflectionMetadataEnum"            "ReflectionMetadataEnumItem"       "ReflectionMetadataEnums"
    "ReflectionMetadataEvents"          "ReflectionMetadataFunctions"      "ReflectionMetadataItem"
    "ReflectionMetadataMember"          "ReflectionMetadataProperties"     "ReflectionMetadataYieldFunctions"
    "Region3"                           "Region3int16"                     "RemoteDebuggerServer"
    "RemoteEvent"                       "RemoteFunction"                   "RenderSettings"
    "RenderingTest"                     "ReplicatedFirst"                  "ReplicatedStorage"
    "ReverbSoundEffect"                 "RigidConstraint"                  "RobloxPluginGuiService"
    "RobloxReplicatedStorage"           "RodConstraint"                    "RopeConstraint"
    "RotationCurve"                     "RtMessagingService"               "RunService"
    "RunningAverageItemDouble"          "RunningAverageItemInt"            "RunningAverageTimeIntervalItem"
    "RuntimeScriptService"              "ScreenGui"                        "ScreenshotHud"
    "Script"                            "ScriptChangeService"              "ScriptCloneWatcher"
    "ScriptCloneWatcherHelper"          "ScriptContext"                    "ScriptDebugger"
    "ScriptDocument"                    "ScriptEditorService"              "ScriptRegistrationService"
    "ScriptService"                     "ScrollingFrame"                   "Seat"
    "Selection"                         "SelectionBox"                     "SelectionLasso"
    "SelectionSphere"                   "ServerReplicator"                 "ServerScriptService"
    "ServerStorage"                     "ServiceProvider"                  "SessionService"
    "Shirt"                             "ShirtGraphic"                     "SkateboardController"
    "Sky"                               "SlidingBallConstraint"            "Smoke"
    "SnippetService"                    "SocialService"                    "SolidModelContentProvider"
    "Sound"                             "SoundEffect"                      "SoundGroup"
    "SoundService"                      "Sparkles"                         "SpawnLocation"
    "SpawnerService"                    "SpecialMesh"                      "SphereHandleAdornment"
    "SpotLight"                         "SpringConstraint"                 "StackFrame"
    "StandalonePluginScripts"           "StandardPages"                    "StarterCharacterScripts"
    "StarterGear"                       "StarterGui"                       "StarterPack"
    "StarterPlayer"                     "StarterPlayerScripts"             "Stats"
    "StatsItem"                         "StopWatchReporter"                "StringValue"
    "Studio"                            "StudioAssetService"               "StudioData"
    "StudioDeviceEmulatorService"       "StudioHighDpiService"             "StudioPublishService"
    "StudioScriptDebugEventListener"    "StudioService"                    "StudioTheme"
    "SunRaysEffect"                     "SurfaceAppearance"                "SurfaceGui"
    "SurfaceGuiBase"                    "SurfaceLight"                     "SurfaceSelection"
    "SwimController"                    "TaskScheduler"                    "Team"
    "TeamCreateService"                 "Teams"                            "TeleportAsyncResult"
    "TeleportOptions"                   "TeleportService"                  "TemporaryCageMeshProvider"
    "TemporaryScriptService"            "Terrain"                          "TerrainDetail"
    "TerrainRegion"                     "TestService"                      "TextBox"
    "TextBoxService"                    "TextButton"                       "TextChannel"
    "TextChatCommand"                   "TextChatConfigurations"           "TextChatMessage"
    "TextChatMessageProperties"         "TextChatService"                  "TextFilterResult"
    "TextLabel"                         "TextService"                      "TextSource"
    "Texture"                           "ThirdPartyUserService"            "ThreadState"
    "TimerService"                      "ToastNotificationService"         "Tool"
    "Torque"                            "TorsionSpringConstraint"          "TotalCountTimeIntervalItem"
    "TouchInputService"                 "TouchTransmitter"                 "TracerService"
    "TrackerLodController"              "TrackerStreamAnimation"           "Trail"
    "Translator"                        "TremoloSoundEffect"               "TriangleMeshPart"
    "TrussPart"                         "Tween"                            "TweenBase"
    "TweenInfo"                         "TweenService"                     "UDim"
    "UGCValidationService"              "UIAspectRatioConstraint"          "UIBase"
    "UIComponent"                       "UIConstraint"                     "UICorner"
    "UIGradient"                        "UIGridLayout"                     "UIGridStyleLayout"
    "UILayout"                          "UIListLayout"                     "UIPadding"
    "UIPageLayout"                      "UIScale"                          "UISizeConstraint"
    "UIStroke"                          "UITableLayout"                    "UITextSizeConstraint"
    "UnionOperation"                    "UniversalConstraint"              "UnreliableRemoteEvent"
    "UnvalidatedAssetService"           "UserGameSettings"                 "UserInputService"
    "UserService"                       "UserSettings"                     "UserStorageService"
    "VRService"                         "ValueBase"                        "Vector2"
    "Vector2int16"                      "Vector3"                          "Vector3int16"
    "VectorForce"                       "VehicleController"                "VehicleSeat"
    "VelocityMotor"                     "VersionControlService"            "VideoCaptureService"
    "VideoFrame"                        "ViewportFrame"                    "VirtualInputManager"
    "VirtualUser"                       "VisibilityService"                "Visit"
    "VoiceChannel"                      "VoiceChatInternal"                "VoiceChatService"
    "WedgePart"                         "Weld"                             "WeldConstraint"
    "WireframeHandleAdornment"          "Workspace"                        "WorldModel"
    "WorldRoot"                         "WrapLayer"                        "WrapTarget"
  )
)

(var
  variable_name: (name) @function.builtin
  (#any-of? @function.builtin
    "assert"        "collectgarbage" "elapsedTime"
    "error"         "gcinfo"         "getfenv"
    "getmetatable"  "ipairs"         "loadstring"
    "next"          "newproxy"       "pairs"
    "pcall"         "PluginManager"  "print"
    "printidentity" "rawequal"       "rawget"
    "rawlen"        "rawset"         "require"
    "select"        "setfenv"        "setmetatable"
    "spawn"         "tick"           "time"
    "tonumber"      "tostring"       "type"
    "typeof"        "unpack"         "UserSettings"
    "version"       "warn"           "workspace"
    "xpcall")
)

(var
  .
  (name) @variable.builtin
  (#any-of? @variable.builtin
    "_G"        "_VERSION" "bit32"
    "coroutine" "debug"    "game"
    "math"      "os"       "plugin"
    "script"    "string"   "table"
    "task"      "utf8"     "workspace"
  )
)

(_ 
  table_name: (name) @variable.builtin
  (#eq? @variable.builtin "bit32")
  .
  (key
    field_name: (name) @function.builtin
    (#any-of? @function.builtin 
      "arshift" "lrotate" "lshift" "replace" 
      "rrotate" "rshift" "btest" "bxor" 
      "band" "bnot" "bor" "countlz" 
      "countrz" "extract" "byteswap"
    )
  )?
)

(_ table_name:
  (name) @variable.builtin
  (#eq? @variable.builtin "coroutine")
  .
  (key
    field_name: (name) @function.builtin
    (#any-of? @function.builtin
      "close"  "create"  "isyieldable"
      "resume" "running" "status"
      "wrap"   "yield"
    )
  )?
)

(_
  table_name: (name) @variable.builtin
  (#eq? @variable.builtin "debug")
  .
  (key
    field_name: (name) @function.builtin
    (#any-of? @function.builtin
      "info"       "traceback"           "profilebegin"
      "profileend" "resetmemorycategory" "setmemorycategory"
      "dumpcodesize"
    )
  )?
)

(_
  table_name: (name) @variable.builtin
  (#eq? @variable.builtin "math")
  .
  (key
    field_name: (name) @function.builtin
    (#any-of? @function.builtin
      "abs"        "acos"  "asin"
      "atan"       "atan2" "ceil"
      "clamp"      "cos"   "cosh"
      "deg"        "exp"   "floor"
      "fmod"       "frexp" "ldexp"
      "log"        "log10" "max"
      "min"        "modf"  "noise"
      "pow"        "rad"   "random"
      "randomseed" "round" "sign"
      "sin"        "sinh"  "sqrt"
      "tan"        "tanh"
    )
  )?
)

(_
  table_name: (name) @variable.builtin
  (#eq? @variable.builtin "math")
  .
  (key
    field_name: (name) @constant.builtin
    ; (#match? @constant.builtin "^(huge|pi)$")
    (#any-of? @constant.builtin "huge" "pi")
  )?
)

(_
  table_name: (name) @variable.builtin
  (#eq? @variable.builtin "os")
  .
  (key
    field_name: (name) @function.builtin
    (#any-of? @function.builtin
      "clock" "date" "difftime"
      "time")
  )?
)

(_
  table_name: (name) @variable.builtin
  (#eq? @variable.builtin "string")
  .
  (key
    field_name: (name) @function.builtin
    (#any-of? @function.builtin
      "byte"    "char"     "find"
      "format"  "gmatch"   "gsub"
      "len"     "lower"    "match"
      "pack"    "packsize" "rep"
      "reverse" "split"    "sub"
      "unpack"  "upper")
  )?
)

(_
  table_name: (name) @variable.builtin
  (#eq? @variable.builtin "table")
  .
  (key
    field_name: (name) @function.builtin
    (#any-of? @function.builtin 
      "create" "clear"    "clone" 
      "concat" "foreach"  "foreachi" 
      "find"   "freeze"   "getn" 
      "insert" "isfrozen" "maxn" 
      "move"   "pack"     "remove"
      "sort"   "unpack")
  )?
)

(_
  table_name: (name) @variable.builtin
  (#eq? @variable.builtin "task")
  .
  (key
    field_name: (name) @function.builtin
    (#any-of? @function.builtin 
      "cancel"      "defer"         "delay" 
      "synchronize" "desynchronize" "spawn" 
      "wait")
  )?
)

(_
  table_name: (name) @variable.builtin
  (#eq? @variable.builtin "utf8")
  .
  (key
    field_name: (name) @function.builtin
    (#any-of? @function.builtin 
      "char"         "codepoint"    "codes"
      "graphemes"    "len"          "offset"
      "nfcnormalize" "nfdnormalize")
  )?
)

(_
  table_name: (name) @variable.builtin
  (#eq? @variable.builtin "utf8")
  .
  (key
    field_name: (name) @constant.builtin
    (#eq? @constant.builtin "charpattern")
  )?
)

(_
  table_name: (name) @variable.builtin
  (#eq? @variable.builtin "buffer")
  (key
    field_name: (name) @function.builtin
    (#any-of? @function.builtin
      "create"   "fromstring" "tostring"
      "len"      "copy"       "fill"
      "readi8"   "readu8"     "readi16"
      "readu16"  "readi32"    "readu32"
      "readf32"  "readf64"    "writei8"
      "writeu8"  "writei16"   "writeu16"
      "writei32" "writeu32"   "writef32"
      "writef64" "readstring" "writestring")
  )?
)

(_
  table_name: (name) @variable.builtin
  (#eq? @variable.builtin "vector")
  (key
    field_name: (name) @function.builtin
    (#any-of? @function.builtin
      "create" "magnitude" "normalize"
      "cross"  "dot"       "angle"
      "floor"  "ceil"      "abs"
      "sign"   "clamp"     "max"
      "min"
    )
  )?
)

(_
  table_name: (name) @variable.builtin
  (#eq? @variable.builtin "vector")
  (key
    field_name: (name) @constant.builtin
    (#any-of? @constant.builtin
      "zero" "one"
    )
  )?
)

(_
  table_name: (name) @variable.builtin
  (#eq? @variable.builtin "Content")
  (key
    field_name: (name) @function.builtin
    (#any-of? @function.builtin
      "fromUri" "fromAssetId" "fromObject"
    )
  )?
)

(type_fn_stmt
  body: (_
    [
      (_
        variable_name: (name) @variable.builtin
        (#eq? @variable.builtin "types")
      )
      (_
        table_name: (name) @variable.builtin
        (#eq? @variable.builtin "types")
        (key
          field_name: (name) @function.builtin
          (#any-of? @function.builtin
            ""
          )
        )
      )
    ]
  )
)

(call_stmt
  method_table: (var
    (name) @variable.builtin
    (#eq? @variable.builtin "game")
  )
  method_name: (name) @function.builtin
  (#eq? @function.builtin "GetService")
  (arglist
    .
    (string)? @string.special
    (#any-of? @string.special
      "\"AccountService\""                         "\"AchievementService\""                "\"AdService\""
      "\"AnalyticsService\""                       "\"AnimationClipProvider\""             "\"AnimationFromVideoCreatorService\""
      "\"AnimationFromVideoCreatorStudioService\"" "\"AnnotationsService\""                "\"AppLifecycleObserverService\""
      "\"AppUpdateService\""                       "\"AssetCounterService\""               "\"AssetDeliveryProxy\""
      "\"AssetImportService\""                     "\"AssetManagerService\""               "\"AssetService\""
      "\"AudioFocusService\""                      "\"AvatarChatService\""                 "\"AvatarCreationService\""
      "\"AvatarEditorService\""                    "\"AvatarImportService\""               "\"BadgeService\""
      "\"CoreGui\""                                "\"StarterGui\""                        "\"BrowserService\""
      "\"BulkImportService\""                      "\"CacheableContentProvider\""          "\"HSRDataContentProvider\""
      "\"MeshContentProvider\""                    "\"SolidModelContentProvider\""         "\"CalloutService\""
      "\"CaptureService\""                         "\"ChangeHistoryService\""              "\"Chat\""
      "\"ChatbotUIService\""                       "\"CloudCRUDService\""                  "\"ClusterPacketCache\""
      "\"CollaboratorsService\""                   "\"CollectionService\""                 "\"CommandService\""
      "\"CommerceService\""                        "\"ConfigureServerService\""            "\"ConnectivityService\""
      "\"ContentProvider\""                        "\"ContextActionService\""              "\"ControllerService\""
      "\"ConversationalAIAcceptanceService\""      "\"CookiesService\""                    "\"CorePackages\""
      "\"CoreScriptDebuggingManagerHelper\""       "\"CoreScriptSyncService\""             "\"CreationDBService\""
      "\"CreatorStoreService\""                    "\"CrossDMScriptChangeListener\""       "\"DataModelPatchService\""
      "\"DataStoreService\""                       "\"Debris\""                            "\"DebuggablePluginWatcher\""
      "\"DebuggerConnectionManager\""              "\"DebuggerManager\""                   "\"DebuggerUIService\""
      "\"DeviceIdService\""                        "\"DraftsService\""                     "\"DraggerService\""
      "\"EditableService\""                        "\"EventIngestService\""                "\"ExampleService\""
      "\"ExperienceAuthService\""                  "\"ExperienceNotificationService\""     "\"ExperienceService\""
      "\"ExperienceStateCaptureService\""          "\"FaceAnimatorService\""               "\"FacialAnimationRecordingService\""
      "\"FacialAnimationStreamingServiceV2\""      "\"FlagStandService\""                  "\"FlyweightService\""
      "\"CSGDictionaryService\""                   "\"NonReplicatedCSGDictionaryService\"" "\"FriendService\""
      "\"GamePassService\""                        "\"GamepadService\""                    "\"GenericChallengeService\""
      "\"Geometry\""                               "\"GeometryService\""                   "\"GoogleAnalyticsConfiguration\""
      "\"GroupService\""                           "\"GuiService\""                        "\"GuidRegistryService\""
      "\"HapticService\""                          "\"HeatmapService\""                    "\"HeightmapImporterService\""
      "\"Hopper\""                                 "\"HttpRbxApiService\""                 "\"HttpService\""
      "\"ILegacyStudioBridge\""                    "\"LegacyStudioBridge\""                "\"IXPService\""
      "\"IncrementalPatchBuilder\""                "\"InsertService\""                     "\"InternalSyncService\""
      "\"JointsService\""                          "\"KeyboardService\""                   "\"KeyframeSequenceProvider\""
      "\"LSPFileSyncService\""                     "\"LanguageService\""                   "\"Lighting\""
      "\"LinkingService\""                         "\"LiveScriptingService\""              "\"LocalStorageService\""
      "\"AppStorageService\""                      "\"UserStorageService\""                "\"LocalizationService\""
      "\"LodDataService\""                         "\"LogReporterService\""                "\"LogService\""
      "\"LoginService\""                           "\"LuaWebService\""                     "\"LuauScriptAnalyzerService\""
      "\"MarketplaceService\""                     "\"MaterialGenerationService\""         "\"MaterialService\""
      "\"MemStorageService\""                      "\"MemoryStoreService\""                "\"MessageBusService\""
      "\"MessagingService\""                       "\"MetaBreakpointManager\""             "\"MouseService\""
      "\"NetworkClient\""                          "\"NetworkServer\""                     "\"NetworkSettings\""
      "\"NotificationService\""                    "\"OmniRecommendationsService\""        "\"OpenCloudService\""
      "\"Workspace\""                              "\"PackageService\""                    "\"PackageUIService\""
      "\"PatchBundlerFileWatch\""                  "\"PathfindingService\""                "\"PermissionsService\""
      "\"PhysicsService\""                         "\"PlaceStatsService\""                 "\"PlacesService\""
      "\"PlatformCloudStorageService\""            "\"PlatformFriendsService\""            "\"PlayerEmulatorService\""
      "\"PlayerHydrationService\""                 "\"PlayerViewService\""                 "\"Players\""
      "\"PluginDebugService\""                     "\"PluginGuiService\""                  "\"PluginManagementService\""
      "\"PluginPolicyService\""                    "\"PointsService\""                     "\"PolicyService\""
      "\"ProcessInstancePhysicsService\""          "\"ProximityPromptService\""            "\"PublishService\""
      "\"RbxAnalyticsService\""                    "\"ReflectionService\""                 "\"RemoteCursorService\""
      "\"RemoteDebuggerServer\""                   "\"RenderSettings\""                    "\"ReplicatedFirst\""
      "\"ReplicatedStorage\""                      "\"RibbonNotificationService\""         "\"RobloxPluginGuiService\""
      "\"RobloxReplicatedStorage\""                "\"RobloxServerStorage\""               "\"RomarkRbxAnalyticsService\""
      "\"RomarkService\""                          "\"RtMessagingService\""                "\"RunService\""
      "\"RuntimeScriptService\""                   "\"SafetyService\""                     "\"ScriptChangeService\""
      "\"ScriptCloneWatcher\""                     "\"ScriptCloneWatcherHelper\""          "\"ScriptCommitService\""
      "\"ScriptContext\""                          "\"ScriptEditorService\""               "\"ScriptProfilerService\""
      "\"ScriptRegistrationService\""              "\"ScriptService\""                     "\"Selection\""
      "\"SelectionHighlightManager\""              "\"ServerScriptService\""               "\"ServerStorage\""
      "\"ServiceVisibilityService\""               "\"SessionService\""                    "\"SharedTableRegistry\""
      "\"ShorelineUpgraderService\""               "\"SmoothVoxelsUpgraderService\""       "\"SnippetService\""
      "\"SocialService\""                          "\"SoundService\""                      "\"SpawnerService\""
      "\"StartPageService\""                       "\"StarterPack\""                       "\"StarterPlayer\""
      "\"StartupMessageService\""                  "\"Stats\""                             "\"StopWatchReporter\""
      "\"StreamingService\""                       "\"Studio\""                            "\"StudioAssetService\""
      "\"StudioData\""                             "\"StudioDeviceEmulatorService\""       "\"StudioPublishService\""
      "\"StudioScriptDebugEventListener\""         "\"StudioSdkService\""                  "\"StudioService\""
      "\"StudioUserService\""                      "\"StudioWidgetsService\""              "\"StylingService\""
      "\"TaskScheduler\""                          "\"TeamCreateData\""                    "\"TeamCreatePublishService\""
      "\"TeamCreateService\""                      "\"Teams\""                             "\"TeleportService\""
      "\"TemporaryCageMeshProvider\""              "\"TemporaryScriptService\""            "\"TestService\""
      "\"TextBoxService\""                         "\"TextChatService\""                   "\"TextService\""
      "\"TextureGenerationService\""               "\"ThirdPartyUserService\""             "\"TimerService\""
      "\"ToastNotificationService\""               "\"TouchInputService\""                 "\"TracerService\""
      "\"TutorialService\""                        "\"TweenService\""                      "\"UGCAvatarService\""
      "\"UGCValidationService\""                   "\"UIDragDetectorService\""             "\"UnvalidatedAssetService\""
      "\"UserInputService\""                       "\"UserService\""                       "\"VRService\""
      "\"VRStatusService\""                        "\"VersionControlService\""             "\"VideoCaptureService\""
      "\"VideoService\""                           "\"VirtualInputManager\""               "\"VirtualUser\""
      "\"VisibilityCheckDispatcher\""              "\"Visit\""                             "\"VisualizationModeService\""
      "\"VoiceChatInternal\""                      "\"VoiceChatService\""                  "\"WebViewService\""
    )
    .
  )
)

