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

#[derive(Clone)]
enum ApiError {}

impl ApiError {
    fn display(&self) -> DisplayableApiError<'_> {
        DisplayableApiError(self)
    }
    fn code(&self) -> ErrorCode {
        todo!()
    }
}

struct DisplayableApiError<'a>(&'a ApiError);

impl<'a> Display for DisplayableApiError<'a> {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
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

trait PlainOldData: 'static + Copy + Sized + Send + Sync {}

impl<T: 'static + Copy + Sized + Send + Sync> PlainOldData for T {}

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
        _param1: &str,
        _param2: u32,
        _param3: &str,
        _param4: &str,
        _param5: u32,
        _param6: &str,
        _param7: &str,
        _param8: u64,
        _protocol: &ProtocolConfig,
    ) -> Result<FutureHandle, Self::Err> {
        todo!()
    }

    fn imports(it: Self::ImportTable) -> Result<(), Self::ImportError>;
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
        _param1: &str,
        _param2: u32,
        _param3: &str,
        _param4: &str,
        _param5: u32,
        _param6: &str,
        _param7: &str,
        _param8: u64,
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
                move |_caller: wasmtime::Caller<'_, ModuleContext>,
                      _param1_1: u32,
                      _param1_2: u32,
                      _param2: u32,
                      _param3_1: u32,
                      _param3_2: u32,
                      _param4_1: u32,
                      _param4_2: u32,
                      _param5: u32,
                      _param6_1: u32,
                      _param6_2: u32,
                      _param7_1: u32,
                      _param7_2: u32,
                      _param8: u64,
                      _protocol_ptr: u32,
                      _output_ptr: u32|
                      -> Result<u32, wasmtime::Trap> {
                    let result = Ok(());
                    Self::log_call("start_training", result)
                },
            )
            .map_err(|err| InstantiationError::Import(err))?;
        return Ok(());
    }
}
