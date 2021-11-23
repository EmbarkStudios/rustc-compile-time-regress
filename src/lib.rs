use std::fmt::Display;

struct ModuleContext;

#[derive(Clone, Copy)]
struct ProtocolConfig;
#[derive(Clone, Copy)]
struct FutureHandle(u64);

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
#[non_exhaustive]
#[must_use]
#[repr(u32)]
enum ErrorCode {
    Success = 0,
    InvalidArguments = 5,
    NotFound = 6,
}

trait HostModule<T = Self> {
    fn get(host_context: &mut ModuleContext) -> Result<&mut T, ApiError>;

    fn name() -> &'static str;

    fn log_call(function: &'static str, res: Result<(), ApiError>) -> Result<u32, wasmtime::Trap> {
        match res {
            Ok(_) => Ok(ErrorCode::Success as u32),
            Err(err) => {
                // skip NotFound code as that is used for graceful errors by some APIs
                // maybe should be moved to other mechanism?
                if err.code() != ErrorCode::NotFound {
                    let err_msg = format!(
                        "{} \"{}\" failed: {}",
                        Self::name(),
                        function,
                        err.display()
                    );
                    log::warn!("{}", err_msg);
                }
                Ok(err.code() as u32)
            }
        }
    }

    fn log_infallible_call(
        function: &'static str,
        res: Result<(), wasmtime::Trap>,
    ) -> Result<(), wasmtime::Trap> {
        res.map_err(|trap| {
            let err_msg = format!(
                "{} \"{}\" failed: {}",
                Self::name(),
                function,
                trap.display_reason()
            );
            log::warn!("{}", err_msg);
            trap
        })
    }

    fn log_deprecated_infallible(
        function: &'static str,
        res: Result<(), wasmtime::Trap>,
    ) -> Result<u32, wasmtime::Trap> {
        match res {
            Ok(_) => Ok(ErrorCode::Success as u32),
            Err(trap) => {
                let err_msg = format!(
                    "{} \"{}\" failed: {}",
                    Self::name(),
                    function,
                    trap.display_reason()
                );
                log::warn!("{}", err_msg);
                Err(trap)
            }
        }
    }
}

#[derive(Debug, Clone)]
enum ApiErrorMessage {
    None,
    Static(&'static str),
    Dynamic(String),
}

impl From<&'static str> for ApiErrorMessage {
    fn from(s: &'static str) -> Self {
        if s.is_empty() {
            Self::None
        } else {
            Self::Static(s)
        }
    }
}

impl From<String> for ApiErrorMessage {
    fn from(s: String) -> Self {
        Self::Dynamic(s)
    }
}

impl ApiErrorMessage {
    fn as_str(&self) -> Option<&'_ str> {
        match self {
            Self::None => None,
            Self::Static(s) => Some(s),
            Self::Dynamic(string) => Some(string.as_str()),
        }
    }
}

fn error_display_chain(error: &dyn std::error::Error) -> String {
    let mut s = error.to_string();
    if let Some(source) = error.source() {
        s.push_str(" -> ");
        s.push_str(&error_display_chain(source));
    }
    s
}

#[derive(Clone)]
enum ApiError {
    InvalidArguments { msg: ApiErrorMessage },
}

impl ApiError {
    fn invalid_arguments<M: Into<ApiErrorMessage>>(msg: M) -> Self {
        Self::InvalidArguments { msg: msg.into() }
    }
    fn invalid_arguments_err<E: std::error::Error>(err: E) -> Self {
        Self::InvalidArguments {
            msg: ApiErrorMessage::Dynamic(error_display_chain(&err)),
        }
    }
    fn display(&self) -> DisplayableApiError<'_> {
        DisplayableApiError(self)
    }
    fn code(&self) -> ErrorCode {
        match self {
            Self::InvalidArguments { .. } => ErrorCode::InvalidArguments,
        }
    }
}

struct DisplayableApiError<'a>(&'a ApiError);

impl<'a> Display for DisplayableApiError<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            ApiError::InvalidArguments { msg } => {
                write!(f, "Invalid arguments")?;
                if let Some(msg_str) = msg.as_str() {
                    write!(f, ": {}", msg_str)?;
                }
            }
        }
        Ok(())
    }
}

#[derive(thiserror::Error, Debug)]
enum ModuleError {
    #[error("Failed to create a module instance")]
    Instantiation(#[from] InstantiationError),
}

#[derive(thiserror::Error, Debug)]
enum InstantiationError {
    #[error("Failed Import")]
    Import(#[source] anyhow::Error),
}

type WasmLinker = wasmtime::Linker<ModuleContext>;

fn memory_ptr_mut<T: PlainOldData>(
    memory: &WasmMemoryHandle<'_>,
    ptr: u32,
) -> Result<*mut T, ApiError> {
    let memory_slice = memory.data();

    let start = ptr as usize;
    let end = start + std::mem::size_of::<T>();
    let byte_slice = memory_slice
        .get(start..end)
        .ok_or_else(|| ApiError::invalid_arguments(""))?;

    let raw_ptr: *mut T = byte_slice.as_ptr() as *mut T;
    Ok(raw_ptr)
}

fn memory_ptr<T: PlainOldData>(
    memory: &WasmMemoryHandle<'_>,
    ptr: u32,
) -> Result<*const T, ApiError> {
    memory_ptr_mut::<T>(memory, ptr).map(|ptr| ptr as *const T)
}

fn get_value<'a, T: PlainOldData>(
    memory: &WasmMemoryHandle<'a>,
    ptr: u32,
) -> Result<&'a T, ApiError> {
    Ok(unsafe { &*memory_ptr::<T>(memory, ptr)? })
}

fn get_value_mut<'a, T: PlainOldData>(
    memory: &mut WasmMemoryHandle<'a>,
    ptr: u32,
) -> Result<&'a mut T, ApiError> {
    Ok(unsafe { &mut *memory_ptr_mut::<T>(memory, ptr)? })
}

pub struct MLApiHost;

impl HostModule for MLApiHost {
    fn name() -> &'static str {
        todo!()
    }
    fn get(_host_context: &mut ModuleContext) -> Result<&mut Self, ApiError> {
        todo!()
    }
}

struct WasmMemoryHandle<'a>(&'a mut [u8]);

impl<'a> WasmMemoryHandle<'a> {
    fn from_raw(slice: &'a mut [u8]) -> Self {
        Self(slice)
    }
    fn data(&self) -> &[u8] {
        self.0
    }
}

trait PlainOldData: 'static + Copy + Sized + Send + Sync {}

impl<T: 'static + Copy + Sized + Send + Sync> PlainOldData for T {}

fn memory_slice<'a, T: PlainOldData>(
    memory: &WasmMemoryHandle<'a>,
    ptr: u32,
    len: u32,
) -> Result<&'a [T], ApiError>
where
    T: Copy,
{
    let len = (len as usize)
        .checked_mul(std::mem::size_of::<T>())
        .ok_or_else(|| ApiError::invalid_arguments(""))?;
    let byte_slice = memory
        .data()
        .get(ptr as usize..)
        .and_then(|s| s.get(..len))
        .ok_or_else(|| ApiError::invalid_arguments(""))?;

    let slice: &[T] = unsafe {
        std::slice::from_raw_parts(
            byte_slice.as_ptr().cast::<T>(),
            byte_slice
                .len()
                .checked_div(std::mem::size_of::<T>())
                .ok_or_else(|| ApiError::invalid_arguments(""))?,
        )
    };
    Ok(slice)
}

fn memory_string<'a>(
    memory: &WasmMemoryHandle<'a>,
    ptr: u32,
    len: u32,
) -> Result<&'a str, ApiError> {
    let bytes = memory_slice(memory, ptr, len)?;
    std::str::from_utf8(bytes).map_err(ApiError::invalid_arguments_err)
}

trait Shim<'t> {
    type Err;
    type Memory;
    type Context;
    type ImportTable;
    type ImportError;
    type WasmTrap;

    fn namespace() -> (&'static str, &'static str) {
        ("hello", "world")
    }

    fn start_training_shim(
        &mut self,
        _hive_url: &str,
        _hive_port: u32,
        _game_name: &str,
        _experiment_name: &str,
        _num_remote_w: u32,
        _config: &str,
        _checkpoint: &str,
        _training_duration_in_seconds: u64,
        _protocol: &ProtocolConfig,
    ) -> Result<FutureHandle, Self::Err> {
        todo!()
    }

    fn start_training_export(
        memory: &mut Self::Memory,
        host_context: &mut Self::Context,
        _hive_url_ptr: u32,
        _hive_url_len: u32,
        _hive_port: u32,
        _game_name_ptr: u32,
        _game_name_len: u32,
        _experiment_name_ptr: u32,
        _experiment_name_len: u32,
        _num_remote_w: u32,
        _config_ptr: u32,
        _config_len: u32,
        _checkpoint_ptr: u32,
        _checkpoint_len: u32,
        _training_duration_in_seconds: u64,
        _protocol_ptr: u32,
        _output_ptr: u32,
    ) -> Result<(), Self::Err>;

    fn imports(it: Self::ImportTable) -> Result<(), Self::ImportError>;
}

#[inline]
fn get_host_context_from_caller<'a>(
    caller: &'a mut wasmtime::Caller<'_, ModuleContext>,
) -> Result<(WasmMemoryHandle<'a>, &'a mut ModuleContext), ApiError> {
    let memory = caller
        .get_export("memory")
        .ok_or_else(|| ApiError::invalid_arguments(""))?
        .into_memory()
        .ok_or_else(|| ApiError::invalid_arguments(""))?;
    let (memory, host_ctx) = memory.data_and_store_mut(caller);
    Ok((WasmMemoryHandle::from_raw(memory), host_ctx))
}

impl<'t> Shim<'t> for MLApiHost {
    type Err = ApiError;
    type Memory = WasmMemoryHandle<'t>;
    type Context = ModuleContext;
    type ImportTable = *mut WasmLinker;
    type ImportError = InstantiationError;
    type WasmTrap = wasmtime::Trap;

    fn start_training_shim(
        &mut self,
        _hive_url: &str,
        _hive_port: u32,
        _game_name: &str,
        _experiment_name: &str,
        _num_remote_w: u32,
        _config: &str,
        _checkpoint: &str,
        _training_duration_in_seconds: u64,
        _protocol: &ProtocolConfig,
    ) -> Result<FutureHandle, Self::Err> {
        todo!()
    }

    fn imports(it: Self::ImportTable) -> Result<(), Self::ImportError> {
        let (namespace, prefix) = Self::namespace();
        let wasmtime_linker = unsafe { &mut *it };
        wasmtime_linker
            .func_wrap(
                namespace,
                format!("{}__{}", prefix, "start_training").as_str(),
                move |mut caller: wasmtime::Caller<'_, ModuleContext>,
                      _hive_url_ptr: u32,
                      _hive_url_len: u32,
                      _hive_port: u32,
                      _game_name_ptr: u32,
                      _game_name_len: u32,
                      _experiment_name_ptr: u32,
                      _experiment_name_len: u32,
                      _num_remote_w: u32,
                      _config_ptr: u32,
                      _config_len: u32,
                      _checkpoint_ptr: u32,
                      _checkpoint_len: u32,
                      _training_duration_in_seconds: u64,
                      _protocol_ptr: u32,
                      _output_ptr: u32|
                      -> Result<u32, wasmtime::Trap> {
                    let (mut memory, host_context) = get_host_context_from_caller(&mut caller)
                        .map_err(|err| wasmtime::Trap::new(err.display().to_string()))?;
                    let memory = &mut memory;
                    let result = Self::start_training_export(
                        memory,
                        host_context,
                        _hive_url_ptr,
                        _hive_url_len,
                        _hive_port,
                        _game_name_ptr,
                        _game_name_len,
                        _experiment_name_ptr,
                        _experiment_name_len,
                        _num_remote_w,
                        _config_ptr,
                        _config_len,
                        _checkpoint_ptr,
                        _checkpoint_len,
                        _training_duration_in_seconds,
                        _protocol_ptr,
                        _output_ptr,
                    );
                    Self::log_call("start_training", result)
                },
            )
            .map_err(|err| InstantiationError::Import(err))?;
        return Ok(());
    }

    fn start_training_export(
        memory: &mut Self::Memory,
        host_context: &mut Self::Context,
        _hive_url_ptr: u32,
        _hive_url_len: u32,
        _hive_port: u32,
        _game_name_ptr: u32,
        _game_name_len: u32,
        _experiment_name_ptr: u32,
        _experiment_name_len: u32,
        _num_remote_w: u32,
        _config_ptr: u32,
        _config_len: u32,
        _checkpoint_ptr: u32,
        _checkpoint_len: u32,
        _training_duration_in_seconds: u64,
        _protocol_ptr: u32,
        _output_ptr: u32,
    ) -> Result<(), Self::Err> {
        Self::get(host_context)?
            .start_training_shim(
                memory_string(memory, _hive_url_ptr, _hive_url_len)?,
                _hive_port,
                memory_string(memory, _game_name_ptr, _game_name_len)?,
                memory_string(memory, _experiment_name_ptr, _experiment_name_len)?,
                _num_remote_w,
                memory_string(memory, _config_ptr, _config_len)?,
                memory_string(memory, _checkpoint_ptr, _checkpoint_len)?,
                _training_duration_in_seconds,
                get_value(memory, _protocol_ptr)?,
            )
            .and_then(|res| {
                let output = get_value_mut(memory, _output_ptr)?;
                *output = res;
                Ok(())
            })
    }
}
