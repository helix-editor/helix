; -- Identifier type inference

; VERY QUESTIONABLE: Highlighting of identifiers based on spelling
(exprBinary ((identifier) @constant
 (#match? @constant "^[A-Z][A-Z0-9_]+$|^[a-z]{2,3}[A-Z].+$")))
(exprUnary ((identifier) @constant
 (#match? @constant "^[A-Z][A-Z0-9_]+$|^[a-z]{2,3}[A-Z].+$")))
(assignment rhs: ((identifier) @constant
 (#match? @constant "^[A-Z][A-Z0-9_]+$|^[a-z]{2,3}[A-Z].+$")))
(exprBrackets ((identifier) @constant
 (#match? @constant "^[A-Z][A-Z0-9_]+$|^[a-z]{2,3}[A-Z].+$")))
(exprParens ((identifier) @constant
 (#match? @constant "^[A-Z][A-Z0-9_]+$|^[a-z]{2,3}[A-Z].+$")))
(exprDot rhs: ((identifier) @constant
 (#match? @constant "^[A-Z][A-Z0-9_]+$|^[a-z]{2,3}[A-Z].+$")))
(exprTpl args: ((identifier) @constant
 (#match? @constant "^[A-Z][A-Z0-9_]+$|^[a-z]{2,3}[A-Z].+$")))
(exprArgs ((identifier) @constant
 (#match? @constant "^[A-Z][A-Z0-9_]+$|^[a-z]{2,3}[A-Z].+$")))
(declEnumValue ((identifier) @constant
 (#match? @constant "^[A-Z][A-Z0-9_]+$|^[a-z]{2,3}[A-Z].+$")))
(defaultValue ((identifier) @constant
 (#match? @constant "^[A-Z][A-Z0-9_]+$|^[a-z]{2,3}[A-Z].+$")))

; -- Break, Continue & Exit
; (Not ideal: ideally, there would be a way to check if these special
; identifiers are shadowed by a local variable)
(statement ((identifier) @keyword.control.return
 (#match? @keyword.control.return "^[eE][xX][iI][tT]$")))
(statement (exprCall entity: ((identifier) @keyword.control.return
 (#match? @keyword.control.return "^[eE][xX][iI][tT]$"))))
(statement ((identifier) @keyword.control.repeat
 (#match? @keyword.control.repeat "^[bB][rR][eE][aA][kK]$")))
(statement ((identifier) @keyword.control.repeat
 (#match? @keyword.control.repeat "^[cC][oO][nN][tT][iI][nN][uU][eE]$")))

; -- Heuristic for procedure/function calls without parentheses
; (If a statement consists only of an identifier, assume it's a procedure)
; (This will still not match all procedure calls, and also may produce false
; positives in rare cases, but only for nonsensical code)

(statement (exprDot rhs: (exprTpl entity: (identifier) @function)))
(statement (exprTpl entity: (identifier) @function))
(statement (exprDot rhs: (identifier) @function))
(statement (identifier) @function)

; -- Procedure name in calls with parentheses
; (Pascal doesn't require parentheses for procedure calls, so this will not
; detect all calls)

(inherited) @function

; foo.bar<t>
(exprCall entity: (exprDot rhs: (exprTpl entity: (identifier) @function)))
; foo.bar
(exprCall entity: (exprDot rhs: (identifier) @function))
; foobar<t>
(exprCall entity: (exprTpl entity: (identifier) @function))
; foobar
(exprCall entity: (identifier) @function)

; -- Fields

(declSection (declVars (declVar   name:(identifier) @variable.other.member)))
(declSection (declField name:(identifier) @variable.other.member))
(declClass   (declField name:(identifier) @variable.other.member))
(exprDot rhs: (exprDot)    @variable.other.member)
(exprDot rhs: (identifier) @variable.other.member)

(recInitializerField name:(identifier) @variable.other.member)

; -- Variable & constant declarations
; (This is only questionable because we cannot detect types of identifiers
; declared in other units, so the results will be inconsistent)

(declVar name: (identifier) @variable)
(declConst name: (identifier) @constant)
(declEnumValue name: (identifier) @constant)

; -- Constant usage

[
	(caseLabel)
	(label)
] @constant

(procAttribute (identifier) @constant)
(procExternal (identifier) @constant)

; -- Type usage

(typeref) @type

; -- Exception parameters

(exceptionHandler variable: (identifier) @variable.parameter)

; -- Template parameters

(genericArg	type: (typeref) @type)
(genericArg	name: (identifier) @variable.parameter)

(declProc name: (genericDot lhs: (identifier) @type))
(declType (genericDot (identifier) @type))

(genericDot (genericTpl (identifier) @type))
(genericDot (genericDot (identifier) @type))

(genericTpl entity: (genericDot (identifier) @type))
(genericTpl entity: (identifier) @type)

; -- Function parameters

(declArg name: (identifier) @variable.parameter)

; Treat property declarations like functions

(declProp name: (identifier) @function)
(declProp getter: (identifier) @variable.other.member)
(declProp setter: (identifier) @variable.other.member)

; -- Procedure & function declarations

; foo.bar<t>
(declProc name: (genericDot rhs: (genericTpl entity: (identifier) @function)))
; foo.bar
(declProc name: (genericDot rhs: (identifier) @function))
; foobar<t>
(declProc name: (genericTpl entity: (identifier) @function))
; foobar
(declProc name: (identifier) @function)

; -- Type declaration

(declType name: (genericTpl entity: (identifier) @type))
(declType name: (identifier) @type)

; -- Comments

(comment)         @comment
(pp)              @function.macro

; -- Variables

(exprBinary (identifier) @variable)
(exprUnary (identifier) @variable)
(assignment (identifier) @variable)
(exprBrackets (identifier) @variable)
(exprParens (identifier) @variable)
(exprDot (identifier) @variable)
(exprTpl (identifier) @variable)
(exprArgs (identifier) @variable)
(defaultValue (identifier) @variable)

; -- Literals

(literalNumber)   @constant.builtin.numeric
(literalString)   @string

; -- Builtin constants

[
	(kTrue)
	(kFalse)
] @constant.builtin.boolean

[
	(kNil)
] @constant.builtin

; -- Punctuation & operators

[
	(kOr)
	(kXor)
	(kDiv)
	(kMod)
	(kAnd)
	(kShl)
	(kShr)
	(kNot)
	(kIs)
	(kAs)
	(kIn)
] @keyword.operator

[
	(kDot)
	(kAdd)
	(kSub)
	(kMul)
	(kFdiv)
	(kAssign)
	(kAssignAdd)
	(kAssignSub)
	(kAssignMul)
	(kAssignDiv)
	(kEq)
	(kLt)
	(kLte)
	(kGt)
	(kGte)
	(kNeq)
	(kAt)
	(kHat)
] @operator

[
	".."
] @punctuation.special

[
	";"
	","
	":"
	(kEndDot)
] @punctuation.delimiter

[
	"("
	")"
	"["
	"]"
] @punctuation.bracket

; -- Attributes

(procAttribute (kPublic) @attribute)

[
	(kDefault)
	(kIndex)
	(kNodefault)
	(kStored)

	(kStatic)
	(kVirtual)
	(kAbstract)
	(kSealed)
	(kDynamic)
	(kOverride)
	(kOverload)
	(kReintroduce)
	(kInline)

	(kForward)

	(kStdcall)
	(kCdecl)
	(kCppdecl)
	(kPascal)
	(kRegister)
	(kMwpascal)
	(kExternal)
	(kName)
	(kMessage)
	(kDeprecated)
	(kExperimental)
	(kPlatform)
	(kUnimplemented)
	(kCvar)
	(kExport)
	(kFar)
	(kNear)
	(kSafecall)
	(kAssembler)
	(kNostackframe)
	(kInterrupt)
	(kNoreturn)
	(kIocheck)
	(kLocal)
	(kHardfloat)
	(kSoftfloat)
	(kMs_abi_default)
	(kMs_abi_cdecl)
	(kSaveregisters)
	(kSysv_abi_default)
	(kSysv_abi_cdecl)
	(kVectorcall)
	(kVarargs)
	(kWinapi)
	(kAlias)
	(kDelayed)

	(rttiAttributes)
	(procAttribute)

] @attribute

; -- Keywords
[
	(kProgram)
	(kLibrary)
	(kUnit)
	(kUses)

	(kBegin)
	(kEnd)
	(kAsm)

	(kVar)
	(kThreadvar)
	(kConst)
	(kResourcestring)
	(kConstref)
	(kOut)
	(kType)
	(kLabel)
	(kExports)

	(kAbsolute)

	(kProperty)
	(kRead)
	(kWrite)
	(kImplements)

	(kClass)
	(kInterface)
	(kObject)
	(kRecord)
	(kObjcclass)
	(kObjccategory)
	(kObjcprotocol)
	(kArray)
	(kFile)
	(kString)
	(kSet)
	(kOf)
	(kHelper)
	(kPacked)

	(kInherited)

	(kGeneric)
	(kSpecialize)

	(kFunction)
	(kProcedure)
	(kConstructor)
	(kDestructor)
	(kOperator)
	(kReference)

	(kInterface)
	(kImplementation)
	(kInitialization)
	(kFinalization)

	(kPublished)
	(kPublic)
	(kProtected)
	(kPrivate)
	(kStrict)
	(kRequired)
	(kOptional)

	(kTry)
	(kExcept)
	(kFinally)
	(kRaise)
	(kOn)
	(kCase)
	(kWith)
	(kGoto)
] @keyword

[
	(kFor)
	(kTo)
	(kDownto)
	(kDo)
	(kWhile)
	(kRepeat)
	(kUntil)
] @keyword.control.repeat

[
	(kIf)
	(kThen)
	(kElse)
] @keyword.control.conditional
