//! An implementation of the JSONRPC 2.0 spec types

// Upstream implementation: https://github.com/paritytech/jsonrpc/tree/38af3c9439aa75481805edf6c05c6622a5ab1e70/core/src/types
// Changes from upstream:
// * unused functions (almost all non-trait-implementation functions) have been removed
// * `#[serde(deny_unknown_fields)]` annotations have been removed on response types
//   for compatibility with non-strict language server implementations like Ruby Sorbet
//   (see https://github.com/helix-editor/helix/issues/2786)
// * some variable names have been lengthened for readability

use serde::de::{self, DeserializeOwned, Visitor};
use serde::{Deserialize, Serialize};
use serde_json::Value;

// https://www.jsonrpc.org/specification#error_object
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ErrorCode {
    ParseError,
    InvalidRequest,
    MethodNotFound,
    InvalidParams,
    InternalError,
    ServerError(i64),
}

impl ErrorCode {
    pub fn code(&self) -> i64 {
        match *self {
            ErrorCode::ParseError => -32700,
            ErrorCode::InvalidRequest => -32600,
            ErrorCode::MethodNotFound => -32601,
            ErrorCode::InvalidParams => -32602,
            ErrorCode::InternalError => -32603,
            ErrorCode::ServerError(code) => code,
        }
    }
}

impl From<i64> for ErrorCode {
    fn from(code: i64) -> Self {
        match code {
            -32700 => ErrorCode::ParseError,
            -32600 => ErrorCode::InvalidRequest,
            -32601 => ErrorCode::MethodNotFound,
            -32602 => ErrorCode::InvalidParams,
            -32603 => ErrorCode::InternalError,
            code => ErrorCode::ServerError(code),
        }
    }
}

impl<'de> Deserialize<'de> for ErrorCode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let code: i64 = Deserialize::deserialize(deserializer)?;
        Ok(ErrorCode::from(code))
    }
}

impl Serialize for ErrorCode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_i64(self.code())
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Error {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl Error {
    pub fn invalid_params<M>(message: M) -> Self
    where
        M: Into<String>,
    {
        Error {
            code: ErrorCode::InvalidParams,
            message: message.into(),
            data: None,
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}

impl std::error::Error for Error {}

// https://www.jsonrpc.org/specification#request_object

/// Request ID
#[derive(Debug, PartialEq, Eq, Clone, Hash, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Id {
    Null,
    Num(#[serde(deserialize_with = "deserialize_jsonrpc_id_num")] u64),
    Str(String),
}

fn deserialize_jsonrpc_id_num<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let num = serde_json::Number::deserialize(deserializer)?;

    if let Some(val) = num.as_u64() {
        return Ok(val);
    };

    // Accept floats as long as they represent positive whole numbers.
    // The JSONRPC spec says "Numbers SHOULD NOT contain fractional parts" so we should try to
    // accept them if possible. The JavaScript type system lumps integers and floats together so
    // some languages may serialize integer IDs as floats with a zeroed fractional part.
    // See <https://github.com/helix-editor/helix/issues/12367>.
    if let Some(val) = num
        .as_f64()
        .filter(|f| f.is_sign_positive() && f.fract() == 0.0)
    {
        return Ok(val as u64);
    }

    Err(de::Error::custom(
        "number must be integer or float representing a whole number in valid u64 range",
    ))
}

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Id::Null => f.write_str("null"),
            Id::Num(num) => write!(f, "{}", num),
            Id::Str(string) => f.write_str(string),
        }
    }
}

/// Protocol Version
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum Version {
    V2,
}

impl Serialize for Version {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match *self {
            Version::V2 => serializer.serialize_str("2.0"),
        }
    }
}

struct VersionVisitor;

impl Visitor<'_> for VersionVisitor {
    type Value = Version;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a string")
    }
    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match value {
            "2.0" => Ok(Version::V2),
            _ => Err(de::Error::custom("invalid version")),
        }
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_identifier(VersionVisitor)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Params {
    None,
    Array(Vec<Value>),
    Map(serde_json::Map<String, Value>),
}

impl Params {
    pub fn parse<D>(self) -> Result<D, Error>
    where
        D: DeserializeOwned,
    {
        let value: Value = self.into();
        serde_json::from_value(value)
            .map_err(|err| Error::invalid_params(format!("Invalid params: {}.", err)))
    }

    pub fn is_none(&self) -> bool {
        self == &Params::None
    }
}

impl From<Params> for Value {
    fn from(params: Params) -> Value {
        match params {
            Params::Array(vec) => Value::Array(vec),
            Params::Map(map) => Value::Object(map),
            Params::None => Value::Null,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MethodCall {
    pub jsonrpc: Option<Version>,
    pub method: String,
    #[serde(default = "default_params", skip_serializing_if = "Params::is_none")]
    pub params: Params,
    pub id: Id,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Notification {
    pub jsonrpc: Option<Version>,
    pub method: String,
    #[serde(default = "default_params", skip_serializing_if = "Params::is_none")]
    pub params: Params,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(untagged)]
pub enum Call {
    MethodCall(MethodCall),
    Notification(Notification),
    Invalid {
        // We can attempt to salvage the id out of the invalid request
        // for better debugging
        #[serde(default = "default_id")]
        id: Id,
    },
}

fn default_params() -> Params {
    Params::None
}

fn default_id() -> Id {
    Id::Null
}

impl From<MethodCall> for Call {
    fn from(method_call: MethodCall) -> Self {
        Call::MethodCall(method_call)
    }
}

impl From<Notification> for Call {
    fn from(notification: Notification) -> Self {
        Call::Notification(notification)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(untagged)]
pub enum Request {
    Single(Call),
    Batch(Vec<Call>),
}

// https://www.jsonrpc.org/specification#response_object

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Success {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jsonrpc: Option<Version>,
    pub result: Value,
    pub id: Id,
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
pub struct Failure {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jsonrpc: Option<Version>,
    pub error: Error,
    pub id: Id,
}

// Note that failure comes first because we're not using
// #[serde(deny_unknown_field)]: we want a request that contains
// both `result` and `error` to be a `Failure`.
#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Output {
    Failure(Failure),
    Success(Success),
}

impl From<Output> for Result<Value, Error> {
    fn from(output: Output) -> Self {
        match output {
            Output::Success(success) => Ok(success.result),
            Output::Failure(failure) => Err(failure.error),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Response {
    Single(Output),
    Batch(Vec<Output>),
}

impl From<Failure> for Response {
    fn from(failure: Failure) -> Self {
        Response::Single(Output::Failure(failure))
    }
}

impl From<Success> for Response {
    fn from(success: Success) -> Self {
        Response::Single(Output::Success(success))
    }
}

#[test]
fn method_call_serialize() {
    use serde_json;

    let m = MethodCall {
        jsonrpc: Some(Version::V2),
        method: "update".to_owned(),
        params: Params::Array(vec![Value::from(1), Value::from(2)]),
        id: Id::Num(1),
    };

    let serialized = serde_json::to_string(&m).unwrap();
    assert_eq!(
        serialized,
        r#"{"jsonrpc":"2.0","method":"update","params":[1,2],"id":1}"#
    );
}

#[test]
fn notification_serialize() {
    use serde_json;

    let n = Notification {
        jsonrpc: Some(Version::V2),
        method: "update".to_owned(),
        params: Params::Array(vec![Value::from(1), Value::from(2)]),
    };

    let serialized = serde_json::to_string(&n).unwrap();
    assert_eq!(
        serialized,
        r#"{"jsonrpc":"2.0","method":"update","params":[1,2]}"#
    );
}

#[test]
fn serialize_skip_none_params() {
    use serde_json;

    let m = MethodCall {
        jsonrpc: Some(Version::V2),
        method: "shutdown".to_owned(),
        params: Params::None,
        id: Id::Num(1),
    };

    let serialized = serde_json::to_string(&m).unwrap();
    assert_eq!(
        serialized,
        r#"{"jsonrpc":"2.0","method":"shutdown","id":1}"#
    );

    let n = Notification {
        jsonrpc: Some(Version::V2),
        method: "exit".to_owned(),
        params: Params::None,
    };

    let serialized = serde_json::to_string(&n).unwrap();
    assert_eq!(serialized, r#"{"jsonrpc":"2.0","method":"exit"}"#);
}

#[test]
fn id_deserialize() {
    use serde_json;

    let id = r#"8"#;
    let deserialized: Id = serde_json::from_str(id).unwrap();
    assert_eq!(deserialized, Id::Num(8));

    let id = r#"4.0"#;
    let deserialized: Id = serde_json::from_str(id).unwrap();
    assert_eq!(deserialized, Id::Num(4));

    let id = r#"0.01"#;
    assert!(serde_json::from_str::<Id>(id).is_err());
}

#[test]
fn success_output_deserialize() {
    use serde_json;

    let dso = r#"{"jsonrpc":"2.0","result":1,"id":1}"#;

    let deserialized: Output = serde_json::from_str(dso).unwrap();
    assert_eq!(
        deserialized,
        Output::Success(Success {
            jsonrpc: Some(Version::V2),
            result: Value::from(1),
            id: Id::Num(1)
        })
    );
}

#[test]
fn success_output_deserialize_with_extra_fields() {
    use serde_json;

    // https://github.com/helix-editor/helix/issues/2786
    let dso = r#"{"jsonrpc":"2.0","result":1,"id":1,"requestMethod":"initialize"}"#;

    let deserialized: Output = serde_json::from_str(dso).unwrap();
    assert_eq!(
        deserialized,
        Output::Success(Success {
            jsonrpc: Some(Version::V2),
            result: Value::from(1),
            id: Id::Num(1)
        })
    );
}
