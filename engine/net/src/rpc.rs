//! RPC：方法注册/调用、二进制 `RpcCall`/`RpcResult`。
//!
//! 与 `Message::Rpc` 的关系：
//! - `Message::Rpc { target, method, payload }` 是**网络信封**，没有请求序号/错误字段；
//! - 本模块的 [`RpcCall`]/[`RpcResult`] 是**应用层 RPC 协议**，包含 `seq` 与
//!   `ok/error`，可直接序列化后放进 `Message::Rpc.payload`；
//! - [`RpcRegistry`] 在服务器/客户端本地注册 handler，`call_bytes` 完成"解码→分发→编码结果"。
//!
//! 限制：方法名 ≤ [`MAX_RPC_METHOD_BYTES`]（64），payload ≤ [`MAX_RPC_PAYLOAD_BYTES`]
//! （512，需与 UDP 消息上限 1200 兼容）；错误文本 ≤ [`MAX_RPC_ERROR_BYTES`]（256）。

use std::collections::BTreeMap;
use std::fmt;

use crate::error::RpcError;
use crate::ids::PlayerId;

/// RPC 格式版本。
pub const RPC_VERSION: u8 = 1;
/// 方法名最大字节数。
pub const MAX_RPC_METHOD_BYTES: usize = 64;
/// payload 最大字节数。
pub const MAX_RPC_PAYLOAD_BYTES: usize = 512;
/// 错误文本最大字节数。
pub const MAX_RPC_ERROR_BYTES: usize = 256;
/// 单个 RPC 编码后最大字节数（保守；实际送入 Message::Rpc 前还会受消息上限约束）。
pub const MAX_RPC_BYTES: usize =
    4 + 1 + 4 + 4 + 2 + MAX_RPC_METHOD_BYTES + 2 + MAX_RPC_PAYLOAD_BYTES;
/// `RpcResult` 固定小上限（含错误文本）。
pub const MAX_RPC_RESULT_BYTES: usize =
    4 + 1 + 4 + 1 + 2 + MAX_RPC_PAYLOAD_BYTES + 2 + MAX_RPC_ERROR_BYTES;

/// 一次 RPC 调用。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RpcCall {
    /// 目标玩家；[`PlayerId::NONE`] 表示服务器。
    pub target: PlayerId,
    /// 方法名。
    pub method: String,
    /// 参数（上层序列化）。
    pub payload: Vec<u8>,
    /// 请求序号（调用方生成，响应原样带回）。
    pub seq: u32,
}

impl RpcCall {
    /// 构造。
    pub fn new(target: PlayerId, method: impl Into<String>, payload: Vec<u8>, seq: u32) -> Self {
        Self {
            target,
            method: method.into(),
            payload,
            seq,
        }
    }

    fn validate(&self) -> Result<(), RpcError> {
        if self.method.len() > MAX_RPC_METHOD_BYTES {
            return Err(RpcError::MethodTooLong {
                size: self.method.len(),
                max: MAX_RPC_METHOD_BYTES,
            });
        }
        if self.payload.len() > MAX_RPC_PAYLOAD_BYTES {
            return Err(RpcError::PayloadTooLarge {
                size: self.payload.len(),
                max: MAX_RPC_PAYLOAD_BYTES,
            });
        }
        Ok(())
    }

    /// 编码 `[u32 len][u8 version][u32 target][u32 seq][u16 method_len][method][u16 payload_len][payload]`。
    pub fn encode(&self) -> Result<Vec<u8>, RpcError> {
        self.validate()?;
        let size = 4 + 1 + 4 + 4 + 2 + self.method.len() + 2 + self.payload.len();
        let mut bytes = Vec::with_capacity(size);
        bytes.extend_from_slice(&(size as u32).to_le_bytes());
        bytes.push(RPC_VERSION);
        bytes.extend_from_slice(&self.target.raw().to_le_bytes());
        bytes.extend_from_slice(&self.seq.to_le_bytes());
        bytes.extend_from_slice(&(self.method.len() as u16).to_le_bytes());
        bytes.extend_from_slice(self.method.as_bytes());
        bytes.extend_from_slice(&(self.payload.len() as u16).to_le_bytes());
        bytes.extend_from_slice(&self.payload);
        Ok(bytes)
    }

    /// 解码。
    pub fn decode(bytes: &[u8]) -> Result<Self, RpcError> {
        if bytes.len() < 4 {
            return Err(RpcError::Decode("RpcCall 长度前缀缺失".to_string()));
        }
        let declared = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        if declared != bytes.len() {
            return Err(RpcError::Decode(format!(
                "RpcCall 长度不一致: 声明 {declared}，实际 {}",
                bytes.len()
            )));
        }
        if bytes.len() > MAX_RPC_BYTES {
            return Err(RpcError::Decode(format!(
                "RpcCall 超过上限 {} 字节",
                MAX_RPC_BYTES
            )));
        }
        let version = *bytes
            .get(4)
            .ok_or_else(|| RpcError::Decode("RpcCall 版本缺失".to_string()))?;
        if version != RPC_VERSION {
            return Err(RpcError::Decode(format!("不支持的 RPC 版本: {version}")));
        }
        let target = PlayerId::new(read_u32(bytes, 5)?);
        let seq = read_u32(bytes, 9)?;
        let method_len = read_u16(bytes, 13)? as usize;
        if method_len > MAX_RPC_METHOD_BYTES {
            return Err(RpcError::MethodTooLong {
                size: method_len,
                max: MAX_RPC_METHOD_BYTES,
            });
        }
        let method_start = 15;
        let method_end = method_start + method_len;
        let method_bytes = bytes
            .get(method_start..method_end)
            .ok_or_else(|| RpcError::Decode("RpcCall method 截断".to_string()))?;
        let method = String::from_utf8(method_bytes.to_vec())
            .map_err(|_| RpcError::Decode("RpcCall method 非 UTF-8".to_string()))?;
        let payload_len = read_u16(bytes, method_end)? as usize;
        if payload_len > MAX_RPC_PAYLOAD_BYTES {
            return Err(RpcError::PayloadTooLarge {
                size: payload_len,
                max: MAX_RPC_PAYLOAD_BYTES,
            });
        }
        let payload_start = method_end + 2;
        let payload = bytes
            .get(payload_start..payload_start + payload_len)
            .ok_or_else(|| RpcError::Decode("RpcCall payload 截断".to_string()))?
            .to_vec();
        if payload_start + payload_len != bytes.len() {
            return Err(RpcError::Decode("RpcCall 尾部多余字节".to_string()));
        }
        Ok(Self {
            target,
            method,
            payload,
            seq,
        })
    }
}

/// RPC 调用结果。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RpcResult {
    /// 对应请求的 seq。
    pub seq: u32,
    /// 是否成功。
    pub ok: bool,
    /// 成功时的返回值。
    pub payload: Vec<u8>,
    /// 失败时的可读错误。
    pub error: Option<String>,
}

impl RpcResult {
    /// 成功结果。
    pub fn ok(seq: u32, payload: Vec<u8>) -> Self {
        Self {
            seq,
            ok: true,
            payload,
            error: None,
        }
    }

    /// 失败结果。
    pub fn error(seq: u32, error: impl Into<String>) -> Self {
        Self {
            seq,
            ok: false,
            payload: Vec::new(),
            error: Some(error.into()),
        }
    }

    /// 编码 `[u32 len][u8 version][u32 seq][u8 ok][u16 payload_len][payload][u16 error_len][error]`。
    pub fn encode(&self) -> Result<Vec<u8>, RpcError> {
        if self.payload.len() > MAX_RPC_PAYLOAD_BYTES {
            return Err(RpcError::PayloadTooLarge {
                size: self.payload.len(),
                max: MAX_RPC_PAYLOAD_BYTES,
            });
        }
        let error = self.error.as_deref().unwrap_or_default();
        if error.len() > MAX_RPC_ERROR_BYTES {
            return Err(RpcError::HandlerError(format!(
                "错误文本过长: {} > {}",
                error.len(),
                MAX_RPC_ERROR_BYTES
            )));
        }
        let size = 4 + 1 + 4 + 1 + 2 + self.payload.len() + 2 + error.len();
        if size > MAX_RPC_RESULT_BYTES {
            return Err(RpcError::Decode(format!(
                "RpcResult 超过上限 {MAX_RPC_RESULT_BYTES} 字节"
            )));
        }
        let mut bytes = Vec::with_capacity(size);
        bytes.extend_from_slice(&(size as u32).to_le_bytes());
        bytes.push(RPC_VERSION);
        bytes.extend_from_slice(&self.seq.to_le_bytes());
        bytes.push(u8::from(self.ok));
        bytes.extend_from_slice(&(self.payload.len() as u16).to_le_bytes());
        bytes.extend_from_slice(&self.payload);
        bytes.extend_from_slice(&(error.len() as u16).to_le_bytes());
        bytes.extend_from_slice(error.as_bytes());
        Ok(bytes)
    }

    /// 解码。
    pub fn decode(bytes: &[u8]) -> Result<Self, RpcError> {
        if bytes.len() < 4 {
            return Err(RpcError::Decode("RpcResult 长度前缀缺失".to_string()));
        }
        let declared = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        if declared != bytes.len() {
            return Err(RpcError::Decode(format!(
                "RpcResult 长度不一致: 声明 {declared}，实际 {}",
                bytes.len()
            )));
        }
        if bytes.len() > MAX_RPC_RESULT_BYTES {
            return Err(RpcError::Decode("RpcResult 超过上限".to_string()));
        }
        let version = *bytes
            .get(4)
            .ok_or_else(|| RpcError::Decode("RpcResult 版本缺失".to_string()))?;
        if version != RPC_VERSION {
            return Err(RpcError::Decode(format!("不支持的 RPC 版本: {version}")));
        }
        let seq = read_u32(bytes, 5)?;
        let ok = *bytes
            .get(9)
            .ok_or_else(|| RpcError::Decode("RpcResult ok 缺失".to_string()))?
            != 0;
        let payload_len = read_u16(bytes, 10)? as usize;
        if payload_len > MAX_RPC_PAYLOAD_BYTES {
            return Err(RpcError::PayloadTooLarge {
                size: payload_len,
                max: MAX_RPC_PAYLOAD_BYTES,
            });
        }
        let payload_start = 12;
        let payload = bytes
            .get(payload_start..payload_start + payload_len)
            .ok_or_else(|| RpcError::Decode("RpcResult payload 截断".to_string()))?
            .to_vec();
        let error_len = read_u16(bytes, payload_start + payload_len)? as usize;
        if error_len > MAX_RPC_ERROR_BYTES {
            return Err(RpcError::Decode("RpcResult error 过长".to_string()));
        }
        let error_start = payload_start + payload_len + 2;
        let error_bytes = bytes
            .get(error_start..error_start + error_len)
            .ok_or_else(|| RpcError::Decode("RpcResult error 截断".to_string()))?;
        let error = if error_len == 0 {
            None
        } else {
            Some(
                String::from_utf8(error_bytes.to_vec())
                    .map_err(|_| RpcError::Decode("RpcResult error 非 UTF-8".to_string()))?,
            )
        };
        if error_start + error_len != bytes.len() {
            return Err(RpcError::Decode("RpcResult 尾部多余字节".to_string()));
        }
        Ok(Self {
            seq,
            ok,
            payload,
            error,
        })
    }
}

type RpcHandler = Box<dyn Fn(&[u8]) -> Result<Vec<u8>, RpcError>>;

/// RPC 方法注册表。
#[derive(Default)]
pub struct RpcRegistry {
    handlers: BTreeMap<String, RpcHandler>,
}

impl fmt::Debug for RpcRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RpcRegistry")
            .field("methods", &self.handlers.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl RpcRegistry {
    /// 创建空注册表。
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册方法；同名方法报 [`RpcError::DuplicateMethod`]。
    pub fn register<F>(&mut self, method: impl Into<String>, handler: F) -> Result<(), RpcError>
    where
        F: Fn(&[u8]) -> Result<Vec<u8>, RpcError> + 'static,
    {
        let method = method.into();
        if method.len() > MAX_RPC_METHOD_BYTES {
            return Err(RpcError::MethodTooLong {
                size: method.len(),
                max: MAX_RPC_METHOD_BYTES,
            });
        }
        if self.handlers.contains_key(&method) {
            return Err(RpcError::DuplicateMethod(method));
        }
        self.handlers.insert(method, Box::new(handler));
        Ok(())
    }

    /// 是否注册。
    pub fn has_method(&self, method: &str) -> bool {
        self.handlers.contains_key(method)
    }

    /// 方法数量。
    pub fn method_count(&self) -> usize {
        self.handlers.len()
    }

    /// 已注册方法名（排序）。
    pub fn methods(&self) -> Vec<&str> {
        self.handlers.keys().map(String::as_str).collect()
    }

    /// 调用；未知方法/过大 payload 返回可读 [`RpcError`]，handler 业务错误包在
    /// [`RpcResult`] 的 `ok=false` 中。
    pub fn call(&self, call: &RpcCall) -> Result<RpcResult, RpcError> {
        call.validate()?;
        let Some(handler) = self.handlers.get(&call.method) else {
            return Err(RpcError::UnknownMethod(call.method.clone()));
        };
        match handler(&call.payload) {
            Ok(payload) => Ok(RpcResult::ok(call.seq, payload)),
            Err(error) => Ok(RpcResult::error(call.seq, error.to_string())),
        }
    }

    /// 便捷：调用并返回 result payload；handler 错误转成 `Err(RpcError::HandlerError)`。
    pub fn call_payload(&self, call: &RpcCall) -> Result<Vec<u8>, RpcError> {
        let result = self.call(call)?;
        if result.ok {
            Ok(result.payload)
        } else {
            Err(RpcError::HandlerError(
                result.error.unwrap_or_else(|| "handler 失败".to_string()),
            ))
        }
    }

    /// 解码 `RpcCall` 字节、分发、编码 `RpcResult` 字节。
    pub fn call_bytes(&self, bytes: &[u8]) -> Result<Vec<u8>, RpcError> {
        let call = RpcCall::decode(bytes)?;
        let result = self.call(&call)?;
        result.encode()
    }
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, RpcError> {
    let raw = bytes
        .get(offset..offset + 2)
        .ok_or_else(|| RpcError::Decode("RPC 数据截断".to_string()))?;
    Ok(u16::from_le_bytes([raw[0], raw[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, RpcError> {
    let raw = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| RpcError::Decode("RPC 数据截断".to_string()))?;
    Ok(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> RpcRegistry {
        let mut registry = RpcRegistry::new();
        registry
            .register("echo", |payload| Ok(payload.to_vec()))
            .unwrap();
        registry
            .register("add", |payload| {
                if payload.len() != 8 {
                    return Err(RpcError::Decode("add 需要 2 个 i32".to_string()));
                }
                let a = i32::from_le_bytes(payload[..4].try_into().unwrap());
                let b = i32::from_le_bytes(payload[4..].try_into().unwrap());
                Ok((a + b).to_le_bytes().to_vec())
            })
            .unwrap();
        registry
            .register("fail", |_| {
                Err(RpcError::HandlerError("业务失败".to_string()))
            })
            .unwrap();
        registry
    }

    #[test]
    fn call_roundtrip_has_seq_and_payload() {
        let call = RpcCall::new(PlayerId::new(7), "echo", b"hello".to_vec(), 42);
        let bytes = call.encode().unwrap();
        assert_eq!(RpcCall::decode(&bytes).unwrap(), call);
        let result = RpcResult::ok(call.seq, b"world".to_vec());
        let result_bytes = result.encode().unwrap();
        assert_eq!(RpcResult::decode(&result_bytes).unwrap(), result);
    }

    #[test]
    fn registry_dispatches_registered_methods() {
        let registry = registry();
        assert_eq!(registry.method_count(), 3);
        let echo = RpcCall::new(PlayerId::NONE, "echo", b"abc".to_vec(), 1);
        let result = registry.call(&echo).unwrap();
        assert!(result.ok);
        assert_eq!(result.payload, b"abc");
        assert_eq!(result.seq, 1);

        let add = RpcCall::new(
            PlayerId::NONE,
            "add",
            [7i32.to_le_bytes(), 5i32.to_le_bytes()].concat(),
            2,
        );
        let result = registry.call(&add).unwrap();
        assert!(result.ok);
        assert_eq!(i32::from_le_bytes(result.payload.try_into().unwrap()), 12);
    }

    #[test]
    fn unknown_method_and_oversized_payload_are_readable_errors() {
        let registry = registry();
        let unknown = RpcCall::new(PlayerId::NONE, "nope", Vec::new(), 1);
        assert!(matches!(
            registry.call(&unknown),
            Err(RpcError::UnknownMethod(method)) if method == "nope"
        ));

        let oversized = RpcCall::new(
            PlayerId::NONE,
            "echo",
            vec![0u8; MAX_RPC_PAYLOAD_BYTES + 1],
            1,
        );
        assert!(matches!(
            registry.call(&oversized),
            Err(RpcError::PayloadTooLarge { .. })
        ));
        assert!(RpcCall::decode(&[1, 2, 3]).is_err());
    }

    #[test]
    fn handler_error_becomes_failed_result_and_call_bytes_roundtrip() {
        let registry = registry();
        let call = RpcCall::new(PlayerId::NONE, "fail", Vec::new(), 9);
        let result = registry.call(&call).unwrap();
        assert!(!result.ok);
        assert_eq!(result.seq, 9);
        assert!(result.error.unwrap().contains("业务失败"));

        let echo = RpcCall::new(PlayerId::NONE, "echo", b"x".to_vec(), 3);
        let request = echo.encode().unwrap();
        let response = registry.call_bytes(&request).unwrap();
        let decoded = RpcResult::decode(&response).unwrap();
        assert!(decoded.ok);
        assert_eq!(decoded.payload, b"x");
    }

    #[test]
    fn duplicate_registration_and_limits_are_rejected() {
        let mut registry = RpcRegistry::new();
        registry
            .register("echo", |payload| Ok(payload.to_vec()))
            .unwrap();
        assert!(matches!(
            registry.register("echo", |payload| Ok(payload.to_vec())),
            Err(RpcError::DuplicateMethod(_))
        ));
        let long_method = "m".repeat(MAX_RPC_METHOD_BYTES + 1);
        assert!(matches!(
            registry.register(long_method, |_| Ok(Vec::new())),
            Err(RpcError::MethodTooLong { .. })
        ));
    }
}
