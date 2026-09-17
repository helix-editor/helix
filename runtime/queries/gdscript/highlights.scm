;; ================ FALLBACKS ================

; Anything unidentified = variable
(identifier) @variable
(name) @variable

; PascalCase = type
((identifier) @type
	(#match? @type "^[A-Z][A-Za-z0-9]*$"))
((name) @type
	(#match? @type "^[A-Z][A-Za-z0-9]*$"))

; CONSTANT_CASE = constant
((identifier) @constant
	(#match? @constant "^[A-Z][A-Z_0-9]*$"))
((name) @constant
	(#match? @constant "^[A-Z][A-Z_0-9]*$"))

;; ================ LITERALS ================

(comment) @comment.line ; # Comment
((comment) @comment.line.documentation ; ## Doc comment
	(#match? @comment.line.documentation "^##"))
(string_name) @string.special.symbol ; &"StringName"
(node_path) @string.special ; ^"Node/Path"
(get_node) @string.special.path ; $Node/Reference
(string) @string
(array) @punctuation.bracket
(dictionary) @punctuation.bracket
(float) @constant.numeric.float
(integer) @constant.numeric.integer
(null) @constant.builtin
[
	(true)
	(false)
] @constant.builtin.boolean

;; ================ KEYWORDS ================

"func" @keyword.function
"return" @keyword.control.return
"await" @keyword.directive
"const" @keyword.storage.type
"var" @keyword.storage.type
"enum" @keyword.storage.type
"class" @keyword.storage.type
"signal" @keyword
"extends" @keyword
"class_name" @keyword
"pass" @keyword
"export" @keyword.control.import

[
	"if"
	"elif"
	"else"
	"match"
	"when"
] @keyword.control.conditional

[
	"for"
	"while"
	"break"
	"continue"
] @keyword.control.repeat

[
	"and"
	"as"
	"in"
	"is"
	"not"
	"or"
] @keyword.operator

;; ================ ANNOTATIONS / ATTRIBUTES ================

(annotation) @attribute
(annotation (identifier) @attribute)

;; ================ DECLARATIONS ================

(function_definition (name) @function)
(constructor_definition) @constructor

(class_definition (name) @type)
(class_name_statement (name) @type)

(const_statement (name) @constant)
(const_statement (identifier) @constant)

(variable_statement (name) @variable.other.member)
(variable_statement (identifier) @variable.other.member)

(type (identifier) @type)
(type) @type

(parameters (identifier) @variable.parameter)
(typed_parameter (identifier) @variable.parameter)

;; ================ FUNCTION CALLS ================

(call (identifier) @function.method)
(attribute_call (identifier) @function.method)
(attribute_call) @function.method

;; ================ GODOT BUILT-INS ================

; Built-in types
((identifier) @type.builtin
	(#any-of? @type.builtin
		"bool" "float" "int"

		"AABB" "AESContext" "AStar2D" "AStar3D" "AStarGrid2D" "AcceptDialog" "AimModifier3D"
		"AnimatableBody2D" "AnimatableBody3D" "AnimatedSprite2D" "AnimatedSprite3D" "AnimatedTexture"
		"Animation" "AnimationLibrary" "AnimationMixer" "AnimationNode" "AnimationNodeAdd2"
		"AnimationNodeAdd3" "AnimationNodeAnimation" "AnimationNodeBlend2" "AnimationNodeBlend3"
		"AnimationNodeBlendSpace1D" "AnimationNodeBlendSpace2D" "AnimationNodeBlendTree"
		"AnimationNodeExtension" "AnimationNodeOneShot" "AnimationNodeOutput"
		"AnimationNodeStateMachine" "AnimationNodeStateMachinePlayback"
		"AnimationNodeStateMachineTransition" "AnimationNodeSub2" "AnimationNodeSync"
		"AnimationNodeTimeScale" "AnimationNodeTimeSeek" "AnimationNodeTransition" "AnimationPlayer"
		"AnimationRootNode" "AnimationTree" "Area2D" "Area3D" "Array" "ArrayMesh" "ArrayOccluder3D"
		"AspectRatioContainer" "AtlasTexture" "AudioBusLayout" "AudioEffect" "AudioEffectAmplify"
		"AudioEffectBandLimitFilter" "AudioEffectBandPassFilter" "AudioEffectCapture"
		"AudioEffectChorus" "AudioEffectCompressor" "AudioEffectDelay" "AudioEffectDistortion"
		"AudioEffectEQ" "AudioEffectEQ10" "AudioEffectEQ21" "AudioEffectEQ6" "AudioEffectFilter"
		"AudioEffectHardLimiter" "AudioEffectHighPassFilter" "AudioEffectHighShelfFilter"
		"AudioEffectInstance" "AudioEffectLimiter" "AudioEffectLowPassFilter"
		"AudioEffectLowShelfFilter" "AudioEffectNotchFilter" "AudioEffectPanner" "AudioEffectPhaser"
		"AudioEffectPitchShift" "AudioEffectRecord" "AudioEffectReverb" "AudioEffectSpectrumAnalyzer"
		"AudioEffectSpectrumAnalyzerInstance" "AudioEffectStereoEnhance" "AudioListener2D"
		"AudioListener3D" "AudioSample" "AudioSamplePlayback" "AudioServer" "AudioStream"
		"AudioStreamGenerator" "AudioStreamGeneratorPlayback" "AudioStreamMicrophone"
		"AudioStreamPlayback" "AudioStreamPlaybackPolyphonic" "AudioStreamPlaybackResampled"
		"AudioStreamPlayer" "AudioStreamPlayer2D" "AudioStreamPlayer3D" "AudioStreamPolyphonic"
		"AudioStreamRandomizer" "AudioStreamWAV" "BackBufferCopy" "BaseButton" "BaseMaterial3D" "Basis"
		"BitMap" "Bone2D" "BoneAttachment3D" "BoneConstraint3D" "BoneMap" "BoneTwistDisperser3D"
		"BoxContainer" "BoxMesh" "BoxOccluder3D" "BoxShape3D" "Button" "ButtonGroup" "CCDIK3D"
		"CPUParticles2D" "CPUParticles3D" "Callable" "CallbackTweener" "Camera2D" "Camera3D"
		"CameraAttributes" "CameraAttributesPhysical" "CameraAttributesPractical" "CameraFeed"
		"CameraServer" "CameraTexture" "CanvasGroup" "CanvasItem" "CanvasItemMaterial" "CanvasLayer"
		"CanvasModulate" "CanvasTexture" "CapsuleMesh" "CapsuleShape2D" "CapsuleShape3D"
		"CenterContainer" "ChainIK3D" "CharFXTransform" "CharacterBody2D" "CharacterBody3D" "CheckBox"
		"CheckButton" "CircleShape2D" "ClassDB" "CodeEdit" "CodeHighlighter" "CollisionObject2D"
		"CollisionObject3D" "CollisionPolygon2D" "CollisionPolygon3D" "CollisionShape2D"
		"CollisionShape3D" "Color" "ColorPalette" "ColorPicker" "ColorPickerButton" "ColorRect"
		"Compositor" "CompositorEffect" "CompressedCubemap" "CompressedCubemapArray"
		"CompressedTexture2D" "CompressedTexture2DArray" "CompressedTexture3D"
		"CompressedTextureLayered" "ConcavePolygonShape2D" "ConcavePolygonShape3D" "ConeTwistJoint3D"
		"ConfigFile" "ConfirmationDialog" "Container" "Control" "ConvertTransformModifier3D"
		"ConvexPolygonShape2D" "ConvexPolygonShape3D" "CopyTransformModifier3D" "Crypto" "CryptoKey"
		"Cubemap" "CubemapArray" "Curve" "Curve2D" "Curve3D" "CurveTexture" "CurveXYZTexture"
		"CylinderMesh" "CylinderShape3D" "DPITexture" "DTLSServer" "DampedSpringJoint2D" "Decal"
		"Dictionary" "DirAccess" "DirectionalLight2D" "DirectionalLight3D" "DisplayServer"
		"EditorCommandPalette" "EditorContextMenuPlugin" "EditorDebuggerPlugin" "EditorDebuggerSession"
		"EditorDock" "EditorExportPlatform" "EditorExportPlatformAppleEmbedded"
		"EditorExportPlatformExtension" "EditorExportPlatformPC" "EditorExportPlugin"
		"EditorExportPreset" "EditorFeatureProfile" "EditorFileDialog" "EditorFileSystem"
		"EditorFileSystemDirectory" "EditorFileSystemImportFormatSupportQuery" "EditorImportPlugin"
		"EditorInspector" "EditorInspectorPlugin" "EditorInterface" "EditorNode3DGizmo"
		"EditorNode3DGizmoPlugin" "EditorPaths" "EditorPlugin" "EditorProperty"
		"EditorResourceConversionPlugin" "EditorResourcePicker" "EditorResourcePreview"
		"EditorResourcePreviewGenerator" "EditorResourceTooltipPlugin" "EditorSceneFormatImporter"
		"EditorScenePostImport" "EditorScenePostImportPlugin" "EditorScript" "EditorScriptPicker"
		"EditorSelection" "EditorSettings" "EditorSpinSlider" "EditorSyntaxHighlighter" "EditorToaster"
		"EditorTranslationParserPlugin" "EditorUndoRedoManager" "EditorVCSInterface" "EncodedObjectAsID"
		"Engine" "EngineDebugger" "EngineProfiler" "Environment" "Expression" "ExternalTexture"
		"FABRIK3D" "FileAccess" "FileDialog" "FileSystemDock" "FlowContainer" "FogMaterial" "FogVolume"
		"FoldableContainer" "FoldableGroup" "Font" "FontFile" "FontVariation" "FramebufferCacheRD"
		"GDExtension" "GDExtensionManager" "GPUParticles2D" "GPUParticles3D" "GPUParticlesAttractor3D"
		"GPUParticlesAttractorBox3D" "GPUParticlesAttractorSphere3D"
		"GPUParticlesAttractorVectorField3D" "GPUParticlesCollision3D" "GPUParticlesCollisionBox3D"
		"GPUParticlesCollisionHeightField3D" "GPUParticlesCollisionSDF3D"
		"GPUParticlesCollisionSphere3D" "Generic6DOFJoint3D" "Geometry2D" "Geometry3D"
		"GeometryInstance3D" "GodotInstance" "Gradient" "GradientTexture1D" "GradientTexture2D"
		"GraphEdit" "GraphElement" "GraphFrame" "GraphNode" "GridContainer" "GrooveJoint2D"
		"HBoxContainer" "HFlowContainer" "HMACContext" "HScrollBar" "HSeparator" "HSlider"
		"HSplitContainer" "HTTPClient" "HTTPRequest" "HashingContext" "HeightMapShape3D" "HingeJoint3D"
		"IKModifier3D" "IP" "Image" "ImageFormatLoader" "ImageFormatLoaderExtension" "ImageTexture"
		"ImageTexture3D" "ImageTextureLayered" "ImmediateMesh" "ImporterMesh" "ImporterMeshInstance3D"
		"Input" "InputEvent" "InputEventAction" "InputEventFromWindow" "InputEventGesture"
		"InputEventJoypadButton" "InputEventJoypadMotion" "InputEventKey" "InputEventMIDI"
		"InputEventMagnifyGesture" "InputEventMouse" "InputEventMouseButton" "InputEventMouseMotion"
		"InputEventPanGesture" "InputEventScreenDrag" "InputEventScreenTouch" "InputEventShortcut"
		"InputEventWithModifiers" "InputMap" "InstancePlaceholder" "IntervalTweener" "ItemList"
		"IterateIK3D" "JNISingleton" "JSON" "JSONRPC" "JacobianIK3D" "JavaClass" "JavaClassWrapper"
		"JavaObject" "JavaScriptBridge" "JavaScriptObject" "Joint2D" "Joint3D" "JointLimitation3D"
		"JointLimitationCone3D" "KinematicCollision2D" "KinematicCollision3D" "Label" "Label3D"
		"LabelSettings" "Light2D" "Light3D" "LightOccluder2D" "LightmapGI" "LightmapGIData"
		"LightmapProbe" "Lightmapper" "LightmapperRD" "LimitAngularVelocityModifier3D" "Line2D"
		"LineEdit" "LinkButton" "Logger" "LookAtModifier3D" "MainLoop" "MarginContainer" "Marker2D"
		"Marker3D" "Marshalls" "Material" "MenuBar" "MenuButton" "Mesh"
		"MeshConvexDecompositionSettings" "MeshDataTool" "MeshInstance2D" "MeshInstance3D" "MeshLibrary"
		"MeshTexture" "MethodTweener" "MissingNode" "MissingResource" "ModifierBoneTarget3D"
		"MovieWriter" "MultiMesh" "MultiMeshInstance2D" "MultiMeshInstance3D" "MultiplayerAPI"
		"MultiplayerAPIExtension" "MultiplayerPeer" "MultiplayerPeerExtension" "Mutex" "NativeMenu"
		"NavigationAgent2D" "NavigationAgent3D" "NavigationLink2D" "NavigationLink3D" "NavigationMesh"
		"NavigationMeshGenerator" "NavigationMeshSourceGeometryData2D"
		"NavigationMeshSourceGeometryData3D" "NavigationObstacle2D" "NavigationObstacle3D"
		"NavigationPathQueryParameters2D" "NavigationPathQueryParameters3D"
		"NavigationPathQueryResult2D" "NavigationPathQueryResult3D" "NavigationPolygon"
		"NavigationRegion2D" "NavigationRegion3D" "NavigationServer2D" "NavigationServer2DManager"
		"NavigationServer3D" "NavigationServer3DManager" "NinePatchRect" "Node" "Node2D" "Node3D"
		"Node3DGizmo" "NodePath" "ORMMaterial3D" "OS" "Object" "Occluder3D" "OccluderInstance3D"
		"OccluderPolygon2D" "OmniLight3D" "OptimizedTranslation" "OptionButton" "PCKPacker"
		"PackedByteArray" "PackedColorArray" "PackedDataContainer" "PackedDataContainerRef"
		"PackedFloat32Array" "PackedFloat64Array" "PackedInt32Array" "PackedInt64Array" "PackedScene"
		"PackedStringArray" "PackedVector2Array" "PackedVector3Array" "PackedVector4Array" "PacketPeer"
		"PacketPeerDTLS" "PacketPeerExtension" "PacketPeerStream" "PacketPeerUDP" "Panel"
		"PanelContainer" "PanoramaSkyMaterial" "Parallax2D" "ParallaxBackground" "ParallaxLayer"
		"ParticleProcessMaterial" "Path2D" "Path3D" "PathFollow2D" "PathFollow3D" "Performance"
		"PhysicalBone2D" "PhysicalBone3D" "PhysicalBoneSimulator3D" "PhysicalSkyMaterial"
		"PhysicsBody2D" "PhysicsBody3D" "PhysicsDirectBodyState2D" "PhysicsDirectBodyState2DExtension"
		"PhysicsDirectBodyState3D" "PhysicsDirectBodyState3DExtension" "PhysicsDirectSpaceState2D"
		"PhysicsDirectSpaceState2DExtension" "PhysicsDirectSpaceState3D"
		"PhysicsDirectSpaceState3DExtension" "PhysicsMaterial" "PhysicsPointQueryParameters2D"
		"PhysicsPointQueryParameters3D" "PhysicsRayQueryParameters2D" "PhysicsRayQueryParameters3D"
		"PhysicsServer2D" "PhysicsServer2DExtension" "PhysicsServer2DManager" "PhysicsServer3D"
		"PhysicsServer3DExtension" "PhysicsServer3DManager" "PhysicsServer3DRenderingServerHandler"
		"PhysicsShapeQueryParameters2D" "PhysicsShapeQueryParameters3D" "PhysicsTestMotionParameters2D"
		"PhysicsTestMotionParameters3D" "PhysicsTestMotionResult2D" "PhysicsTestMotionResult3D"
		"PinJoint2D" "PinJoint3D" "PlaceholderCubemap" "PlaceholderCubemapArray" "PlaceholderMaterial"
		"PlaceholderMesh" "PlaceholderTexture2D" "PlaceholderTexture2DArray" "PlaceholderTexture3D"
		"PlaceholderTextureLayered" "Plane" "PlaneMesh" "PointLight2D" "PointMesh" "Polygon2D"
		"PolygonOccluder3D" "PolygonPathFinder" "Popup" "PopupMenu" "PopupPanel"
		"PortableCompressedTexture2D" "PrimitiveMesh" "PrismMesh" "ProceduralSkyMaterial" "ProgressBar"
		"ProjectSettings" "Projection" "PropertyTweener" "QuadMesh" "QuadOccluder3D" "Quaternion"
		"RDAttachmentFormat" "RDFramebufferPass" "RDPipelineColorBlendState"
		"RDPipelineColorBlendStateAttachment" "RDPipelineDepthStencilState" "RDPipelineMultisampleState"
		"RDPipelineRasterizationState" "RDPipelineSpecializationConstant" "RDSamplerState"
		"RDShaderFile" "RDShaderSPIRV" "RDShaderSource" "RDTextureFormat" "RDTextureView" "RDUniform"
		"RDVertexAttribute" "RID" "RandomNumberGenerator" "Range" "RayCast2D" "RayCast3D" "Rect2"
		"Rect2i" "RectangleShape2D" "RefCounted" "ReferenceRect" "ReflectionProbe" "RemoteTransform2D"
		"RemoteTransform3D" "RenderData" "RenderDataExtension" "RenderDataRD" "RenderSceneBuffers"
		"RenderSceneBuffersConfiguration" "RenderSceneBuffersExtension" "RenderSceneBuffersRD"
		"RenderSceneData" "RenderSceneDataExtension" "RenderSceneDataRD" "RenderingDevice"
		"RenderingServer" "Resource" "ResourceFormatLoader" "ResourceFormatSaver" "ResourceImporter"
		"ResourceImporterBMFont" "ResourceImporterBitMap" "ResourceImporterCSVTranslation"
		"ResourceImporterDynamicFont" "ResourceImporterImage" "ResourceImporterImageFont"
		"ResourceImporterLayeredTexture" "ResourceImporterOBJ" "ResourceImporterSVG"
		"ResourceImporterScene" "ResourceImporterShaderFile" "ResourceImporterTexture"
		"ResourceImporterTextureAtlas" "ResourceImporterWAV" "ResourceLoader" "ResourcePreloader"
		"ResourceSaver" "ResourceUID" "RetargetModifier3D" "RibbonTrailMesh" "RichTextEffect"
		"RichTextLabel" "RigidBody2D" "RigidBody3D" "RootMotionView" "SceneState" "SceneTree"
		"SceneTreeTimer" "Script" "ScriptBacktrace" "ScriptCreateDialog" "ScriptEditor"
		"ScriptEditorBase" "ScriptExtension" "ScriptLanguage" "ScriptLanguageExtension" "ScrollBar"
		"ScrollContainer" "SegmentShape2D" "Semaphore" "SeparationRayShape2D" "SeparationRayShape3D"
		"Separator" "Shader" "ShaderGlobalsOverride" "ShaderInclude" "ShaderIncludeDB" "ShaderMaterial"
		"Shape2D" "Shape3D" "ShapeCast2D" "ShapeCast3D" "Shortcut" "Signal" "Skeleton2D" "Skeleton3D"
		"SkeletonIK3D" "SkeletonModification2D" "SkeletonModification2DCCDIK"
		"SkeletonModification2DFABRIK" "SkeletonModification2DJiggle" "SkeletonModification2DLookAt"
		"SkeletonModification2DPhysicalBones" "SkeletonModification2DStackHolder"
		"SkeletonModification2DTwoBoneIK" "SkeletonModificationStack2D" "SkeletonModifier3D"
		"SkeletonProfile" "SkeletonProfileHumanoid" "Skin" "SkinReference" "Sky" "Slider"
		"SliderJoint3D" "SocketServer" "SoftBody3D" "SphereMesh" "SphereOccluder3D" "SphereShape3D"
		"SpinBox" "SplineIK3D" "SplitContainer" "SpotLight3D" "SpringArm3D" "SpringBoneCollision3D"
		"SpringBoneCollisionCapsule3D" "SpringBoneCollisionPlane3D" "SpringBoneCollisionSphere3D"
		"SpringBoneSimulator3D" "Sprite2D" "Sprite3D" "SpriteBase3D" "SpriteFrames" "StandardMaterial3D"
		"StaticBody2D" "StaticBody3D" "StatusIndicator" "StreamPeer" "StreamPeerBuffer"
		"StreamPeerExtension" "StreamPeerGZIP" "StreamPeerSocket" "StreamPeerTCP" "StreamPeerTLS"
		"StreamPeerUDS" "String" "StringName" "StyleBox" "StyleBoxEmpty" "StyleBoxFlat" "StyleBoxLine"
		"StyleBoxTexture" "SubViewport" "SubViewportContainer" "SubtweenTweener" "SurfaceTool"
		"SyntaxHighlighter" "SystemFont" "TCPServer" "TLSOptions" "TabBar" "TabContainer" "TextEdit"
		"TextLine" "TextMesh" "TextParagraph" "TextServer" "TextServerDummy" "TextServerExtension"
		"TextServerManager" "Texture" "Texture2D" "Texture2DArray" "Texture2DArrayRD" "Texture2DRD"
		"Texture3D" "Texture3DRD" "TextureButton" "TextureCubemapArrayRD" "TextureCubemapRD"
		"TextureLayered" "TextureLayeredRD" "TextureProgressBar" "TextureRect" "Theme" "ThemeDB"
		"Thread" "TileData" "TileMap" "TileMapLayer" "TileMapPattern" "TileSet" "TileSetAtlasSource"
		"TileSetScenesCollectionSource" "TileSetSource" "Time" "Timer" "TorusMesh" "TouchScreenButton"
		"Transform2D" "Transform3D" "Translation" "TranslationDomain" "TranslationServer" "Tree"
		"TreeItem" "TriangleMesh" "TubeTrailMesh" "Tween" "Tweener" "TwoBoneIK3D" "UDPServer"
		"UDSServer" "UndoRedo" "UniformSetCacheRD" "VBoxContainer" "VFlowContainer" "VScrollBar"
		"VSeparator" "VSlider" "VSplitContainer" "Variant" "Vector2" "Vector2i" "Vector3" "Vector3i"
		"Vector4" "Vector4i" "VehicleBody3D" "VehicleWheel3D" "VideoStream" "VideoStreamPlayback"
		"VideoStreamPlayer" "Viewport" "ViewportTexture" "VisibleOnScreenEnabler2D"
		"VisibleOnScreenEnabler3D" "VisibleOnScreenNotifier2D" "VisibleOnScreenNotifier3D"
		"VisualInstance3D" "VisualShader" "VisualShaderNode" "VisualShaderNodeBillboard"
		"VisualShaderNodeBooleanConstant" "VisualShaderNodeBooleanParameter" "VisualShaderNodeClamp"
		"VisualShaderNodeColorConstant" "VisualShaderNodeColorFunc" "VisualShaderNodeColorOp"
		"VisualShaderNodeColorParameter" "VisualShaderNodeComment" "VisualShaderNodeCompare"
		"VisualShaderNodeConstant" "VisualShaderNodeCubemap" "VisualShaderNodeCubemapParameter"
		"VisualShaderNodeCurveTexture" "VisualShaderNodeCurveXYZTexture" "VisualShaderNodeCustom"
		"VisualShaderNodeDerivativeFunc" "VisualShaderNodeDeterminant" "VisualShaderNodeDistanceFade"
		"VisualShaderNodeDotProduct" "VisualShaderNodeExpression" "VisualShaderNodeFaceForward"
		"VisualShaderNodeFloatConstant" "VisualShaderNodeFloatFunc" "VisualShaderNodeFloatOp"
		"VisualShaderNodeFloatParameter" "VisualShaderNodeFrame" "VisualShaderNodeFresnel"
		"VisualShaderNodeGlobalExpression" "VisualShaderNodeGroupBase" "VisualShaderNodeIf"
		"VisualShaderNodeInput" "VisualShaderNodeIntConstant" "VisualShaderNodeIntFunc"
		"VisualShaderNodeIntOp" "VisualShaderNodeIntParameter" "VisualShaderNodeIs"
		"VisualShaderNodeLinearSceneDepth" "VisualShaderNodeMix" "VisualShaderNodeMultiplyAdd"
		"VisualShaderNodeOuterProduct" "VisualShaderNodeOutput" "VisualShaderNodeParameter"
		"VisualShaderNodeParameterRef" "VisualShaderNodeParticleAccelerator"
		"VisualShaderNodeParticleBoxEmitter" "VisualShaderNodeParticleConeVelocity"
		"VisualShaderNodeParticleEmit" "VisualShaderNodeParticleEmitter"
		"VisualShaderNodeParticleMeshEmitter" "VisualShaderNodeParticleMultiplyByAxisAngle"
		"VisualShaderNodeParticleOutput" "VisualShaderNodeParticleRandomness"
		"VisualShaderNodeParticleRingEmitter" "VisualShaderNodeParticleSphereEmitter"
		"VisualShaderNodeProximityFade" "VisualShaderNodeRandomRange" "VisualShaderNodeRemap"
		"VisualShaderNodeReroute" "VisualShaderNodeResizableBase" "VisualShaderNodeRotationByAxis"
		"VisualShaderNodeSDFRaymarch" "VisualShaderNodeSDFToScreenUV" "VisualShaderNodeSample3D"
		"VisualShaderNodeScreenNormalWorldSpace" "VisualShaderNodeScreenUVToSDF"
		"VisualShaderNodeSmoothStep" "VisualShaderNodeStep" "VisualShaderNodeSwitch"
		"VisualShaderNodeTexture" "VisualShaderNodeTexture2DArray"
		"VisualShaderNodeTexture2DArrayParameter" "VisualShaderNodeTexture2DParameter"
		"VisualShaderNodeTexture3D" "VisualShaderNodeTexture3DParameter"
		"VisualShaderNodeTextureParameter" "VisualShaderNodeTextureParameterTriplanar"
		"VisualShaderNodeTextureSDF" "VisualShaderNodeTextureSDFNormal"
		"VisualShaderNodeTransformCompose" "VisualShaderNodeTransformConstant"
		"VisualShaderNodeTransformDecompose" "VisualShaderNodeTransformFunc"
		"VisualShaderNodeTransformOp" "VisualShaderNodeTransformParameter"
		"VisualShaderNodeTransformVecMult" "VisualShaderNodeUIntConstant" "VisualShaderNodeUIntFunc"
		"VisualShaderNodeUIntOp" "VisualShaderNodeUIntParameter" "VisualShaderNodeUVFunc"
		"VisualShaderNodeUVPolarCoord" "VisualShaderNodeVarying" "VisualShaderNodeVaryingGetter"
		"VisualShaderNodeVaryingSetter" "VisualShaderNodeVec2Constant" "VisualShaderNodeVec2Parameter"
		"VisualShaderNodeVec3Constant" "VisualShaderNodeVec3Parameter" "VisualShaderNodeVec4Constant"
		"VisualShaderNodeVec4Parameter" "VisualShaderNodeVectorBase" "VisualShaderNodeVectorCompose"
		"VisualShaderNodeVectorDecompose" "VisualShaderNodeVectorDistance" "VisualShaderNodeVectorFunc"
		"VisualShaderNodeVectorLen" "VisualShaderNodeVectorOp" "VisualShaderNodeVectorRefract"
		"VisualShaderNodeWorldPositionFromDepth" "VoxelGI" "VoxelGIData" "WeakRef" "Window"
		"WorkerThreadPool" "World2D" "World3D" "WorldBoundaryShape2D" "WorldBoundaryShape3D"
		"WorldEnvironment" "X509Certificate" "XMLParser" "XRAnchor3D" "XRBodyModifier3D" "XRBodyTracker"
		"XRCamera3D" "XRController3D" "XRControllerTracker" "XRFaceModifier3D" "XRFaceTracker"
		"XRHandModifier3D" "XRHandTracker" "XRInterface" "XRInterfaceExtension" "XRNode3D" "XROrigin3D"
		"XRPose" "XRPositionalTracker" "XRServer" "XRTracker" "XRVRS"

		"AudioServer" "CameraServer" "ClassDB" "DisplayServer" "EditorInterface" "Engine"
		"EngineDebugger" "GDExtensionManager" "Geometry2D" "Geometry3D" "GodotSharp" "IP" "Input"
		"InputMap" "JavaClassWrapper" "JavaScriptBridge" "Marshalls" "NavigationMeshGenerator"
		"NavigationServer2D" "NavigationServer3D" "OS" "Performance" "PhysicsServer2D"
		"PhysicsServer2DManager" "PhysicsServer3D" "PhysicsServer3DManager" "ProjectSettings"
		"RenderingServer" "ResourceLoader" "ResourceSaver" "ResourceUID" "TextServerManager" "ThemeDB"
		"Time" "TranslationServer" "WorkerThreadPool" "XRServer"
	))

; Built-in functions
((identifier) @function.builtin
	(#any-of? @function.builtin
		"abs" "absf" "absi" "acos" "acosh" "angle_difference" "asin" "asinh" "atan" "atan2" "atanh"
		"bezier_derivative" "bezier_interpolate" "bytes_to_var" "bytes_to_var_with_objects" "ceil"
		"ceilf" "ceili" "clamp" "clampf" "clampi" "cos" "cosh" "cubic_interpolate"
		"cubic_interpolate_angle" "cubic_interpolate_angle_in_time" "cubic_interpolate_in_time"
		"db_to_linear" "deg_to_rad" "ease" "error_string" "exp" "floor" "floorf" "floori" "fmod"
		"fposmod" "hash" "instance_from_id" "inverse_lerp" "is_equal_approx" "is_finite" "is_inf"
		"is_instance_id_valid" "is_instance_valid" "is_nan" "is_same" "is_zero_approx" "lerp"
		"lerp_angle" "lerpf" "linear_to_db" "log" "max" "maxf" "maxi" "min" "minf" "mini" "move_toward"
		"nearest_po2" "pingpong" "posmod" "pow" "print" "print_rich" "print_verbose" "printerr"
		"printraw" "prints" "printt" "push_error" "push_warning" "rad_to_deg" "rand_from_seed" "randf"
		"randf_range" "randfn" "randi" "randi_range" "randomize" "remap" "rid_allocate_id"
		"rid_from_int64" "rotate_toward" "round" "roundf" "roundi" "seed" "sign" "signf" "signi" "sin"
		"sinh" "smoothstep" "snapped" "snappedf" "snappedi" "sqrt" "step_decimals" "str" "str_to_var"
		"tan" "tanh" "type_convert" "type_string" "typeof" "var_to_bytes" "var_to_bytes_with_objects"
		"var_to_str" "weakref" "wrap" "wrapf" "wrapi"

		"Color8" "assert" "char" "convert" "dict_to_inst" "get_stack" "inst_to_dict" "is_instance_of"
		"len" "load" "ord" "preload" "print_debug" "print_stack" "range" "type_exists"
	))

; Built-in constants
((identifier) @constant.builtin
	(#any-of? @constant.builtin
		"INF" "NAN" "PI" "TAU"

		"CLOCKWISE" "CORNER_BOTTOM_LEFT" "CORNER_BOTTOM_RIGHT" "CORNER_TOP_LEFT" "CORNER_TOP_RIGHT"
		"COUNTERCLOCKWISE" "ERR_ALREADY_EXISTS" "ERR_ALREADY_IN_USE" "ERR_BUG" "ERR_BUSY"
		"ERR_CANT_ACQUIRE_RESOURCE" "ERR_CANT_CONNECT" "ERR_CANT_CREATE" "ERR_CANT_FORK" "ERR_CANT_OPEN"
		"ERR_CANT_RESOLVE" "ERR_COMPILATION_FAILED" "ERR_CONNECTION_ERROR" "ERR_CYCLIC_LINK"
		"ERR_DATABASE_CANT_READ" "ERR_DATABASE_CANT_WRITE" "ERR_DOES_NOT_EXIST" "ERR_DUPLICATE_SYMBOL"
		"ERR_FILE_ALREADY_IN_USE" "ERR_FILE_BAD_DRIVE" "ERR_FILE_BAD_PATH" "ERR_FILE_CANT_OPEN"
		"ERR_FILE_CANT_READ" "ERR_FILE_CANT_WRITE" "ERR_FILE_CORRUPT" "ERR_FILE_EOF"
		"ERR_FILE_MISSING_DEPENDENCIES" "ERR_FILE_NO_PERMISSION" "ERR_FILE_NOT_FOUND"
		"ERR_FILE_UNRECOGNIZED" "ERR_HELP" "ERR_INVALID_DATA" "ERR_INVALID_DECLARATION"
		"ERR_INVALID_PARAMETER" "ERR_LINK_FAILED" "ERR_LOCKED" "ERR_METHOD_NOT_FOUND"
		"ERR_OUT_OF_MEMORY" "ERR_PARAMETER_RANGE_ERROR" "ERR_PARSE_ERROR" "ERR_PRINTER_ON_FIRE"
		"ERR_QUERY_FAILED" "ERR_SCRIPT_FAILED" "ERR_SKIP" "ERR_TIMEOUT" "ERR_UNAUTHORIZED"
		"ERR_UNAVAILABLE" "ERR_UNCONFIGURED" "EULER_ORDER_XYZ" "EULER_ORDER_XZY" "EULER_ORDER_YXZ"
		"EULER_ORDER_YZX" "EULER_ORDER_ZXY" "EULER_ORDER_ZYX" "FAILED" "HORIZONTAL"
		"HORIZONTAL_ALIGNMENT_CENTER" "HORIZONTAL_ALIGNMENT_FILL" "HORIZONTAL_ALIGNMENT_LEFT"
		"HORIZONTAL_ALIGNMENT_RIGHT" "INLINE_ALIGNMENT_BASELINE_TO" "INLINE_ALIGNMENT_BOTTOM"
		"INLINE_ALIGNMENT_BOTTOM_TO" "INLINE_ALIGNMENT_CENTER" "INLINE_ALIGNMENT_CENTER_TO"
		"INLINE_ALIGNMENT_IMAGE_MASK" "INLINE_ALIGNMENT_TEXT_MASK" "INLINE_ALIGNMENT_TO_BASELINE"
		"INLINE_ALIGNMENT_TO_BOTTOM" "INLINE_ALIGNMENT_TO_CENTER" "INLINE_ALIGNMENT_TOP"
		"INLINE_ALIGNMENT_TOP_TO" "INLINE_ALIGNMENT_TO_TOP" "JOY_AXIS_INVALID" "JOY_AXIS_LEFT_X"
		"JOY_AXIS_LEFT_Y" "JOY_AXIS_MAX" "JOY_AXIS_RIGHT_X" "JOY_AXIS_RIGHT_Y" "JOY_AXIS_SDL_MAX"
		"JOY_AXIS_TRIGGER_LEFT" "JOY_AXIS_TRIGGER_RIGHT" "JOY_BUTTON_A" "JOY_BUTTON_B" "JOY_BUTTON_BACK"
		"JOY_BUTTON_DPAD_DOWN" "JOY_BUTTON_DPAD_LEFT" "JOY_BUTTON_DPAD_RIGHT" "JOY_BUTTON_DPAD_UP"
		"JOY_BUTTON_GUIDE" "JOY_BUTTON_INVALID" "JOY_BUTTON_LEFT_SHOULDER" "JOY_BUTTON_LEFT_STICK"
		"JOY_BUTTON_MAX" "JOY_BUTTON_MISC1" "JOY_BUTTON_PADDLE1" "JOY_BUTTON_PADDLE2"
		"JOY_BUTTON_PADDLE3" "JOY_BUTTON_PADDLE4" "JOY_BUTTON_RIGHT_SHOULDER" "JOY_BUTTON_RIGHT_STICK"
		"JOY_BUTTON_SDL_MAX" "JOY_BUTTON_START" "JOY_BUTTON_TOUCHPAD" "JOY_BUTTON_X" "JOY_BUTTON_Y"
		"KEY_0" "KEY_1" "KEY_2" "KEY_3" "KEY_4" "KEY_5" "KEY_6" "KEY_7" "KEY_8" "KEY_9" "KEY_A"
		"KEY_ALT" "KEY_AMPERSAND" "KEY_APOSTROPHE" "KEY_ASCIICIRCUM" "KEY_ASCIITILDE" "KEY_ASTERISK"
		"KEY_AT" "KEY_B" "KEY_BACK" "KEY_BACKSLASH" "KEY_BACKSPACE" "KEY_BACKTAB" "KEY_BAR"
		"KEY_BRACELEFT" "KEY_BRACERIGHT" "KEY_BRACKETLEFT" "KEY_BRACKETRIGHT" "KEY_C" "KEY_CAPSLOCK"
		"KEY_CLEAR" "KEY_CODE_MASK" "KEY_COLON" "KEY_COMMA" "KEY_CTRL" "KEY_D" "KEY_DELETE" "KEY_DOLLAR"
		"KEY_DOWN" "KEY_E" "KEY_END" "KEY_ENTER" "KEY_EQUAL" "KEY_ESCAPE" "KEY_EXCLAM" "KEY_F" "KEY_F1"
		"KEY_F10" "KEY_F11" "KEY_F12" "KEY_F13" "KEY_F14" "KEY_F15" "KEY_F16" "KEY_F17" "KEY_F18"
		"KEY_F19" "KEY_F2" "KEY_F20" "KEY_F21" "KEY_F22" "KEY_F23" "KEY_F24" "KEY_F25" "KEY_F26"
		"KEY_F27" "KEY_F28" "KEY_F29" "KEY_F3" "KEY_F30" "KEY_F31" "KEY_F32" "KEY_F33" "KEY_F34"
		"KEY_F35" "KEY_F4" "KEY_F5" "KEY_F6" "KEY_F7" "KEY_F8" "KEY_F9" "KEY_FAVORITES" "KEY_FORWARD"
		"KEY_G" "KEY_GLOBE" "KEY_GREATER" "KEY_H" "KEY_HELP" "KEY_HOME" "KEY_HOMEPAGE" "KEY_HYPER"
		"KEY_I" "KEY_INSERT" "KEY_J" "KEY_JIS_EISU" "KEY_JIS_KANA" "KEY_K" "KEY_KEYBOARD" "KEY_KP_0"
		"KEY_KP_1" "KEY_KP_2" "KEY_KP_3" "KEY_KP_4" "KEY_KP_5" "KEY_KP_6" "KEY_KP_7" "KEY_KP_8"
		"KEY_KP_9" "KEY_KP_ADD" "KEY_KP_DIVIDE" "KEY_KP_ENTER" "KEY_KP_MULTIPLY" "KEY_KP_PERIOD"
		"KEY_KP_SUBTRACT" "KEY_L" "KEY_LAUNCH0" "KEY_LAUNCH1" "KEY_LAUNCH2" "KEY_LAUNCH3" "KEY_LAUNCH4"
		"KEY_LAUNCH5" "KEY_LAUNCH6" "KEY_LAUNCH7" "KEY_LAUNCH8" "KEY_LAUNCH9" "KEY_LAUNCHA"
		"KEY_LAUNCHB" "KEY_LAUNCHC" "KEY_LAUNCHD" "KEY_LAUNCHE" "KEY_LAUNCHF" "KEY_LAUNCHMAIL"
		"KEY_LAUNCHMEDIA" "KEY_LEFT" "KEY_LESS" "KEY_LOCATION_LEFT" "KEY_LOCATION_RIGHT"
		"KEY_LOCATION_UNSPECIFIED" "KEY_M" "KEY_MASK_ALT" "KEY_MASK_CMD_OR_CTRL" "KEY_MASK_CTRL"
		"KEY_MASK_GROUP_SWITCH" "KEY_MASK_KPAD" "KEY_MASK_META" "KEY_MASK_SHIFT" "KEY_MEDIANEXT"
		"KEY_MEDIAPLAY" "KEY_MEDIAPREVIOUS" "KEY_MEDIARECORD" "KEY_MEDIASTOP" "KEY_MENU" "KEY_META"
		"KEY_MINUS" "KEY_MODIFIER_MASK" "KEY_N" "KEY_NONE" "KEY_NUMBERSIGN" "KEY_NUMLOCK" "KEY_O"
		"KEY_OPENURL" "KEY_P" "KEY_PAGEDOWN" "KEY_PAGEUP" "KEY_PARENLEFT" "KEY_PARENRIGHT" "KEY_PAUSE"
		"KEY_PERCENT" "KEY_PERIOD" "KEY_PLUS" "KEY_PRINT" "KEY_Q" "KEY_QUESTION" "KEY_QUOTEDBL"
		"KEY_QUOTELEFT" "KEY_R" "KEY_REFRESH" "KEY_RIGHT" "KEY_S" "KEY_SCROLLLOCK" "KEY_SEARCH"
		"KEY_SECTION" "KEY_SEMICOLON" "KEY_SHIFT" "KEY_SLASH" "KEY_SPACE" "KEY_SPECIAL" "KEY_STANDBY"
		"KEY_STOP" "KEY_SYSREQ" "KEY_T" "KEY_TAB" "KEY_U" "KEY_UNDERSCORE" "KEY_UNKNOWN" "KEY_UP"
		"KEY_V" "KEY_VOLUMEDOWN" "KEY_VOLUMEMUTE" "KEY_VOLUMEUP" "KEY_W" "KEY_X" "KEY_Y" "KEY_YEN"
		"KEY_Z" "METHOD_FLAG_CONST" "METHOD_FLAG_EDITOR" "METHOD_FLAG_NORMAL" "METHOD_FLAG_OBJECT_CORE"
		"METHOD_FLAGS_DEFAULT" "METHOD_FLAG_STATIC" "METHOD_FLAG_VARARG" "METHOD_FLAG_VIRTUAL"
		"METHOD_FLAG_VIRTUAL_REQUIRED" "MIDI_MESSAGE_ACTIVE_SENSING" "MIDI_MESSAGE_AFTERTOUCH"
		"MIDI_MESSAGE_CHANNEL_PRESSURE" "MIDI_MESSAGE_CONTINUE" "MIDI_MESSAGE_CONTROL_CHANGE"
		"MIDI_MESSAGE_NONE" "MIDI_MESSAGE_NOTE_OFF" "MIDI_MESSAGE_NOTE_ON" "MIDI_MESSAGE_PITCH_BEND"
		"MIDI_MESSAGE_PROGRAM_CHANGE" "MIDI_MESSAGE_QUARTER_FRAME" "MIDI_MESSAGE_SONG_POSITION_POINTER"
		"MIDI_MESSAGE_SONG_SELECT" "MIDI_MESSAGE_START" "MIDI_MESSAGE_STOP"
		"MIDI_MESSAGE_SYSTEM_EXCLUSIVE" "MIDI_MESSAGE_SYSTEM_RESET" "MIDI_MESSAGE_TIMING_CLOCK"
		"MIDI_MESSAGE_TUNE_REQUEST" "MOUSE_BUTTON_LEFT" "MOUSE_BUTTON_MASK_LEFT"
		"MOUSE_BUTTON_MASK_MB_XBUTTON1" "MOUSE_BUTTON_MASK_MB_XBUTTON2" "MOUSE_BUTTON_MASK_MIDDLE"
		"MOUSE_BUTTON_MASK_RIGHT" "MOUSE_BUTTON_MIDDLE" "MOUSE_BUTTON_NONE" "MOUSE_BUTTON_RIGHT"
		"MOUSE_BUTTON_WHEEL_DOWN" "MOUSE_BUTTON_WHEEL_LEFT" "MOUSE_BUTTON_WHEEL_RIGHT"
		"MOUSE_BUTTON_WHEEL_UP" "MOUSE_BUTTON_XBUTTON1" "MOUSE_BUTTON_XBUTTON2" "OK" "OP_ADD" "OP_AND"
		"OP_BIT_AND" "OP_BIT_NEGATE" "OP_BIT_OR" "OP_BIT_XOR" "OP_DIVIDE" "OP_EQUAL" "OP_GREATER"
		"OP_GREATER_EQUAL" "OP_IN" "OP_LESS" "OP_LESS_EQUAL" "OP_MAX" "OP_MODULE" "OP_MULTIPLY"
		"OP_NEGATE" "OP_NOT" "OP_NOT_EQUAL" "OP_OR" "OP_POSITIVE" "OP_POWER" "OP_SHIFT_LEFT"
		"OP_SHIFT_RIGHT" "OP_SUBTRACT" "OP_XOR" "PROPERTY_HINT_ARRAY_TYPE"
		"PROPERTY_HINT_COLOR_NO_ALPHA" "PROPERTY_HINT_DICTIONARY_TYPE" "PROPERTY_HINT_DIR"
		"PROPERTY_HINT_ENUM" "PROPERTY_HINT_ENUM_SUGGESTION" "PROPERTY_HINT_EXP_EASING"
		"PROPERTY_HINT_EXPRESSION" "PROPERTY_HINT_FILE" "PROPERTY_HINT_FILE_PATH" "PROPERTY_HINT_FLAGS"
		"PROPERTY_HINT_GLOBAL_DIR" "PROPERTY_HINT_GLOBAL_FILE" "PROPERTY_HINT_GLOBAL_SAVE_FILE"
		"PROPERTY_HINT_GROUP_ENABLE" "PROPERTY_HINT_HIDE_QUATERNION_EDIT" "PROPERTY_HINT_INPUT_NAME"
		"PROPERTY_HINT_INT_IS_OBJECTID" "PROPERTY_HINT_INT_IS_POINTER"
		"PROPERTY_HINT_LAYERS_2D_NAVIGATION" "PROPERTY_HINT_LAYERS_2D_PHYSICS"
		"PROPERTY_HINT_LAYERS_2D_RENDER" "PROPERTY_HINT_LAYERS_3D_NAVIGATION"
		"PROPERTY_HINT_LAYERS_3D_PHYSICS" "PROPERTY_HINT_LAYERS_3D_RENDER"
		"PROPERTY_HINT_LAYERS_AVOIDANCE" "PROPERTY_HINT_LINK" "PROPERTY_HINT_LOCALE_ID"
		"PROPERTY_HINT_LOCALIZABLE_STRING" "PROPERTY_HINT_MAX" "PROPERTY_HINT_MULTILINE_TEXT"
		"PROPERTY_HINT_NODE_PATH_TO_EDITED_NODE" "PROPERTY_HINT_NODE_PATH_VALID_TYPES"
		"PROPERTY_HINT_NODE_TYPE" "PROPERTY_HINT_NONE" "PROPERTY_HINT_OBJECT_ID"
		"PROPERTY_HINT_OBJECT_TOO_BIG" "PROPERTY_HINT_ONESHOT" "PROPERTY_HINT_PASSWORD"
		"PROPERTY_HINT_PLACEHOLDER_TEXT" "PROPERTY_HINT_RANGE" "PROPERTY_HINT_RESOURCE_TYPE"
		"PROPERTY_HINT_SAVE_FILE" "PROPERTY_HINT_TOOL_BUTTON" "PROPERTY_HINT_TYPE_STRING"
		"PROPERTY_USAGE_ALWAYS_DUPLICATE" "PROPERTY_USAGE_ARRAY" "PROPERTY_USAGE_CATEGORY"
		"PROPERTY_USAGE_CHECKABLE" "PROPERTY_USAGE_CHECKED" "PROPERTY_USAGE_CLASS_IS_BITFIELD"
		"PROPERTY_USAGE_CLASS_IS_ENUM" "PROPERTY_USAGE_DEFAULT" "PROPERTY_USAGE_DEFERRED_SET_RESOURCE"
		"PROPERTY_USAGE_EDITOR" "PROPERTY_USAGE_EDITOR_BASIC_SETTING"
		"PROPERTY_USAGE_EDITOR_INSTANTIATE_OBJECT" "PROPERTY_USAGE_GROUP" "PROPERTY_USAGE_HIGH_END_GFX"
		"PROPERTY_USAGE_INTERNAL" "PROPERTY_USAGE_KEYING_INCREMENTS" "PROPERTY_USAGE_NEVER_DUPLICATE"
		"PROPERTY_USAGE_NIL_IS_VARIANT" "PROPERTY_USAGE_NODE_PATH_FROM_SCENE_ROOT"
		"PROPERTY_USAGE_NO_EDITOR" "PROPERTY_USAGE_NO_INSTANCE_STATE" "PROPERTY_USAGE_NONE"
		"PROPERTY_USAGE_READ_ONLY" "PROPERTY_USAGE_RESOURCE_NOT_PERSISTENT"
		"PROPERTY_USAGE_RESTART_IF_CHANGED" "PROPERTY_USAGE_SCRIPT_DEFAULT_VALUE"
		"PROPERTY_USAGE_SCRIPT_VARIABLE" "PROPERTY_USAGE_SECRET" "PROPERTY_USAGE_STORAGE"
		"PROPERTY_USAGE_STORE_IF_NULL" "PROPERTY_USAGE_SUBGROUP" "PROPERTY_USAGE_UPDATE_ALL_IF_MODIFIED"
		"SIDE_BOTTOM" "SIDE_LEFT" "SIDE_RIGHT" "SIDE_TOP" "TYPE_AABB" "TYPE_ARRAY" "TYPE_BASIS"
		"TYPE_BOOL" "TYPE_CALLABLE" "TYPE_COLOR" "TYPE_DICTIONARY" "TYPE_FLOAT" "TYPE_INT" "TYPE_MAX"
		"TYPE_NIL" "TYPE_NODE_PATH" "TYPE_OBJECT" "TYPE_PACKED_BYTE_ARRAY" "TYPE_PACKED_COLOR_ARRAY"
		"TYPE_PACKED_FLOAT32_ARRAY" "TYPE_PACKED_FLOAT64_ARRAY" "TYPE_PACKED_INT32_ARRAY"
		"TYPE_PACKED_INT64_ARRAY" "TYPE_PACKED_STRING_ARRAY" "TYPE_PACKED_VECTOR2_ARRAY"
		"TYPE_PACKED_VECTOR3_ARRAY" "TYPE_PACKED_VECTOR4_ARRAY" "TYPE_PLANE" "TYPE_PROJECTION"
		"TYPE_QUATERNION" "TYPE_RECT2" "TYPE_RECT2I" "TYPE_RID" "TYPE_SIGNAL" "TYPE_STRING"
		"TYPE_STRING_NAME" "TYPE_TRANSFORM2D" "TYPE_TRANSFORM3D" "TYPE_VECTOR2" "TYPE_VECTOR2I"
		"TYPE_VECTOR3" "TYPE_VECTOR3I" "TYPE_VECTOR4" "TYPE_VECTOR4I" "VERTICAL"
		"VERTICAL_ALIGNMENT_BOTTOM" "VERTICAL_ALIGNMENT_CENTER" "VERTICAL_ALIGNMENT_FILL"
		"VERTICAL_ALIGNMENT_TOP"
	))

((identifier) @variable.builtin
	(#any-of? @variable.builtin "self" "super"))

;; ================ OPERATORS & PUNCTUATION ================

[
	"("
	")"
	"["
	"]"
	"{"
	"}"
] @punctuation.bracket

[
	","
	"."
	":"
	";"
] @punctuation.delimiter

[
	"+"
	"-"
	"*"
	"/"
	"="
	":="
	"=="
	"!="
	"<"
	">"
	"<="
	">="
	"->"
	"+="
	"-="
	"*="
	"/="
] @operator
