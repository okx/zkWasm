pub mod context;

pub static mut ERROR_CODE_WRAPPER: ErrorCodeWrapper = ErrorCodeWrapper::new();

pub struct ErrorCodeWrapper {
    code: u32,
    index: u32,
}

impl ErrorCodeWrapper {
    pub const fn new() -> Self {
        Self { code: 0, index: 0 }
    }

    pub fn set_code(&mut self, code: u32) {
        self.code = code;
    }
    pub fn set_index(&mut self, index: u32) {
        self.index = index;
    }

    pub fn get_code(&self) -> u32 {
        self.code
    }
}

pub fn set_code_index(code: u32, index: u32) {
    unsafe {
        ERROR_CODE_WRAPPER.set_code(code);
        ERROR_CODE_WRAPPER.set_index(index)
    }
}

pub fn get_code() -> u32 {
    unsafe { ERROR_CODE_WRAPPER.get_code() }
}




