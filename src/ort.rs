//! The few calls of ONNX Runtime's C API that run one model, bound by hand and loaded at run time from the library
//! edits downloads on first use (`tools::onnxruntime`): no crate, nothing linked, the binary stays small. The OrtApi
//! table is append-only, so its first entries keep their places in every release; the places below are counted in
//! onnxruntime_c_api.h (1.31) and the table asked for is version 17, which any runtime since 1.17 serves.
use std::ffi::{c_char, c_void, CStr, CString};
use std::path::Path;

type Status = *mut c_void;
type Ptr = *mut c_void;

/// Places in the OrtApi table.
mod at {
    pub const GET_ERROR_MESSAGE: usize = 2;
    pub const CREATE_ENV: usize = 3;
    pub const CREATE_SESSION_FROM_ARRAY: usize = 8;
    pub const RUN: usize = 9;
    pub const CREATE_SESSION_OPTIONS: usize = 10;
    pub const SET_GRAPH_OPTIMIZATION_LEVEL: usize = 23;
    pub const SET_INTRA_OP_NUM_THREADS: usize = 24;
    pub const CREATE_TENSOR_WITH_DATA: usize = 49;
    pub const GET_TENSOR_MUTABLE_DATA: usize = 51;
    pub const GET_DIMENSIONS_COUNT: usize = 61;
    pub const GET_DIMENSIONS: usize = 62;
    pub const GET_TENSOR_TYPE_AND_SHAPE: usize = 65;
    pub const CREATE_CPU_MEMORY_INFO: usize = 69;
    pub const RELEASE_ENV: usize = 92;
    pub const RELEASE_STATUS: usize = 93;
    pub const RELEASE_MEMORY_INFO: usize = 94;
    pub const RELEASE_SESSION: usize = 95;
    pub const RELEASE_VALUE: usize = 96;
    pub const RELEASE_TENSOR_TYPE_AND_SHAPE_INFO: usize = 99;
    pub const RELEASE_SESSION_OPTIONS: usize = 100;
}

const API_VERSION: u32 = 17;
const LOG_WARNING: i32 = 2;
const ENABLE_ALL: i32 = 99;
const ARENA_ALLOCATOR: i32 = 1;
const MEM_DEFAULT: i32 = 0;
const FLOAT: i32 = 1;

#[repr(C)]
struct ApiBase {
    get_api: unsafe extern "system" fn(u32) -> *const *const c_void,
    version: unsafe extern "system" fn() -> *const c_char,
}

/// A loaded model, ready to run.
pub struct Model {
    _lib: libloading::Library,
    api: *const *const c_void,
    env: Ptr,
    session: Ptr,
    mem: Ptr,
}

/// A tensor ONNX Runtime made (an output), released when dropped.
pub struct Value<'a> {
    m: &'a Model,
    pub raw: Ptr,
}

impl Drop for Value<'_> {
    fn drop(&mut self) {
        unsafe { self.m.release(at::RELEASE_VALUE, self.raw) }
    }
}

impl Value<'_> {
    /// The tensor's numbers (it must hold f32) and its shape.
    pub fn floats(&self) -> Result<(&[f32], Vec<i64>), String> {
        unsafe {
            let mut info: Ptr = std::ptr::null_mut();
            self.m.check(self.m.f::<unsafe extern "system" fn(Ptr, *mut Ptr) -> Status>(at::GET_TENSOR_TYPE_AND_SHAPE)(self.raw, &mut info))?;
            let mut n = 0usize;
            let r = self.m.check(self.m.f::<unsafe extern "system" fn(Ptr, *mut usize) -> Status>(at::GET_DIMENSIONS_COUNT)(info, &mut n));
            let mut dims = vec![0i64; n];
            let r2 = r.and_then(|_| self.m.check(self.m.f::<unsafe extern "system" fn(Ptr, *mut i64, usize) -> Status>(at::GET_DIMENSIONS)(info, dims.as_mut_ptr(), n)));
            self.m.release(at::RELEASE_TENSOR_TYPE_AND_SHAPE_INFO, info);
            r2?;
            let mut data: *mut c_void = std::ptr::null_mut();
            self.m.check(self.m.f::<unsafe extern "system" fn(Ptr, *mut *mut c_void) -> Status>(at::GET_TENSOR_MUTABLE_DATA)(self.raw, &mut data))?;
            let count = dims.iter().map(|d| (*d).max(0) as usize).product::<usize>();
            Ok((std::slice::from_raw_parts(data as *const f32, count), dims))
        }
    }
}

impl Model {
    /// Loads the runtime library at `lib` and the model at `model`.
    pub fn load(lib: &Path, model: &Path) -> Result<Model, String> {
        let bytes = std::fs::read(model).map_err(|e| format!("{}: {e}", model.display()))?;
        unsafe {
            let lib = libloading::Library::new(lib).map_err(|e| format!("could not load ONNX Runtime ({}): {e}", lib.display()))?;
            let base: libloading::Symbol<unsafe extern "system" fn() -> *const ApiBase> = lib.get(b"OrtGetApiBase\0").map_err(|e| e.to_string())?;
            let base = base();
            if base.is_null() {
                return Err("ONNX Runtime gave no API".into());
            }
            let api = ((*base).get_api)(API_VERSION);
            if api.is_null() {
                let v = CStr::from_ptr(((*base).version)()).to_string_lossy().into_owned();
                return Err(format!("ONNX Runtime {v} is too old (1.17 or newer is needed)"));
            }
            let mut m = Model { _lib: lib, api, env: std::ptr::null_mut(), session: std::ptr::null_mut(), mem: std::ptr::null_mut() };
            let name = CString::new("edits").unwrap();
            let mut env: Ptr = std::ptr::null_mut();
            m.check(m.f::<unsafe extern "system" fn(i32, *const c_char, *mut Ptr) -> Status>(at::CREATE_ENV)(LOG_WARNING, name.as_ptr(), &mut env))?;
            m.env = env;
            let (mut opts, mut session): (Ptr, Ptr) = (std::ptr::null_mut(), std::ptr::null_mut());
            m.check(m.f::<unsafe extern "system" fn(*mut Ptr) -> Status>(at::CREATE_SESSION_OPTIONS)(&mut opts))?;
            let made = m.check(m.f::<unsafe extern "system" fn(Ptr, i32) -> Status>(at::SET_GRAPH_OPTIMIZATION_LEVEL)(opts, ENABLE_ALL))
                // 0: one thread per core
                .and_then(|_| m.check(m.f::<unsafe extern "system" fn(Ptr, i32) -> Status>(at::SET_INTRA_OP_NUM_THREADS)(opts, 0)))
                .and_then(|_| m.check(m.f::<unsafe extern "system" fn(Ptr, *const c_void, usize, Ptr, *mut Ptr) -> Status>(at::CREATE_SESSION_FROM_ARRAY)(
                    m.env, bytes.as_ptr() as *const c_void, bytes.len(), opts, &mut session)));
            m.release(at::RELEASE_SESSION_OPTIONS, opts);
            made?;
            m.session = session;
            let mut mem: Ptr = std::ptr::null_mut();
            m.check(m.f::<unsafe extern "system" fn(i32, i32, *mut Ptr) -> Status>(at::CREATE_CPU_MEMORY_INFO)(ARENA_ALLOCATOR, MEM_DEFAULT, &mut mem))?;
            m.mem = mem;
            Ok(m)
        }
    }

    unsafe fn f<T: Copy>(&self, i: usize) -> T {
        let p = *self.api.add(i);
        std::mem::transmute_copy::<*const c_void, T>(&p)
    }

    unsafe fn release(&self, i: usize, p: Ptr) {
        if !p.is_null() {
            self.f::<unsafe extern "system" fn(Ptr)>(i)(p);
        }
    }

    unsafe fn check(&self, s: Status) -> Result<(), String> {
        if s.is_null() {
            return Ok(());
        }
        let msg = CStr::from_ptr(self.f::<unsafe extern "system" fn(Status) -> *const c_char>(at::GET_ERROR_MESSAGE)(s)).to_string_lossy().into_owned();
        self.release(at::RELEASE_STATUS, s);
        Err(format!("ONNX Runtime: {msg}"))
    }

    /// An f32 tensor over `data` (not copied: `data` must outlive it).
    pub fn tensor<'a>(&'a self, data: &'a mut [f32], shape: &[i64]) -> Result<Value<'a>, String> {
        let mut v: Ptr = std::ptr::null_mut();
        unsafe {
            self.check(self.f::<unsafe extern "system" fn(Ptr, *mut c_void, usize, *const i64, usize, i32, *mut Ptr) -> Status>(at::CREATE_TENSOR_WITH_DATA)(
                self.mem, data.as_mut_ptr() as *mut c_void, std::mem::size_of_val(data), shape.as_ptr(), shape.len(), FLOAT, &mut v))?;
        }
        Ok(Value { m: self, raw: v })
    }

    /// Runs the model on named inputs; the named outputs come back in order.
    pub fn run(&self, inputs: &[(&str, &Value)], outputs: &[&str]) -> Result<Vec<Value<'_>>, String> {
        let in_names: Vec<CString> = inputs.iter().map(|i| CString::new(i.0).unwrap()).collect();
        let out_names: Vec<CString> = outputs.iter().map(|o| CString::new(*o).unwrap()).collect();
        let in_ptrs: Vec<*const c_char> = in_names.iter().map(|c| c.as_ptr()).collect();
        let out_ptrs: Vec<*const c_char> = out_names.iter().map(|c| c.as_ptr()).collect();
        let vals: Vec<Ptr> = inputs.iter().map(|i| i.1.raw).collect();
        let mut outs: Vec<Ptr> = vec![std::ptr::null_mut(); outputs.len()];
        unsafe {
            self.check(self.f::<unsafe extern "system" fn(Ptr, Ptr, *const *const c_char, *const Ptr, usize, *const *const c_char, usize, *mut Ptr) -> Status>(at::RUN)(
                self.session, std::ptr::null_mut(), in_ptrs.as_ptr(), vals.as_ptr(), vals.len(), out_ptrs.as_ptr(), outs.len(), outs.as_mut_ptr()))?;
        }
        Ok(outs.into_iter().map(|raw| Value { m: self, raw }).collect())
    }
}

impl Drop for Model {
    fn drop(&mut self) {
        unsafe {
            self.release(at::RELEASE_MEMORY_INFO, self.mem);
            self.release(at::RELEASE_SESSION, self.session);
            self.release(at::RELEASE_ENV, self.env);
        }
    }
}
