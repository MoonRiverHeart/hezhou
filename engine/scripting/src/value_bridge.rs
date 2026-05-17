#[repr(C)]
#[derive(Clone, Copy)]
pub struct ScriptValue {
    pub type_tag: u8,
    pub error_flag: u8,
    pub reserved: u16,
    pub int_value: i32,
    pub float_value: f32,
    pub double_value: f64,
    pub ptr_value: usize,
}

pub enum ScriptTypeTag {
    Null = 0,
    Int = 1,
    Float = 2,
    Double = 3,
    String = 4,
    Object = 5,
    Array = 6,
    AsyncOp = 7,
    TaskId = 8,
}

impl Default for ScriptValue {
    fn default() -> Self {
        Self {
            type_tag: ScriptTypeTag::Null as u8,
            error_flag: 0,
            reserved: 0,
            int_value: 0,
            float_value: 0.0,
            double_value: 0.0,
            ptr_value: 0,
        }
    }
}

impl ScriptValue {
    pub fn null() -> Self {
        Self::default()
    }

    pub fn from_int(value: i32) -> Self {
        Self {
            type_tag: ScriptTypeTag::Int as u8,
            int_value: value,
            ..Default::default()
        }
    }

    pub fn from_float(value: f32) -> Self {
        Self {
            type_tag: ScriptTypeTag::Float as u8,
            float_value: value,
            ..Default::default()
        }
    }

    pub fn from_double(value: f64) -> Self {
        Self {
            type_tag: ScriptTypeTag::Double as u8,
            double_value: value,
            ..Default::default()
        }
    }

    pub fn ok(mut self) -> Self {
        self.error_flag = 0;
        self
    }

    pub fn err(message: &str) -> Self {
        let c_string = std::ffi::CString::new(message).unwrap();
        Self {
            type_tag: ScriptTypeTag::String as u8,
            error_flag: 1,
            ptr_value: c_string.into_raw() as usize,
            ..Default::default()
        }
    }

    pub fn is_ok(&self) -> bool {
        self.error_flag == 0
    }

    pub fn is_err(&self) -> bool {
        self.error_flag == 1
    }

    pub fn get_int(&self) -> Option<i32> {
        if self.is_ok() && self.type_tag == ScriptTypeTag::Int as u8 {
            Some(self.int_value)
        } else {
            None
        }
    }

    pub fn get_float(&self) -> Option<f32> {
        if self.is_ok() && self.type_tag == ScriptTypeTag::Float as u8 {
            Some(self.float_value)
        } else {
            None
        }
    }

    pub fn get_error_message(&self) -> Option<&str> {
        if self.is_err() && self.ptr_value != 0 {
            let c_str = unsafe { std::ffi::CStr::from_ptr(self.ptr_value as *const i8) };
            Some(c_str.to_str().unwrap_or(""))
        } else {
            None
        }
    }
}

unsafe impl Send for ScriptValue {}
unsafe impl Sync for ScriptValue {}
