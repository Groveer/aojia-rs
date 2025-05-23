use once_cell::sync::OnceCell;
use std::ptr;
use windows::{
    Win32::{
        Globalization::GetUserDefaultLCID,
        System::{
            Com::{
                CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
                CoUninitialize, DISPATCH_METHOD, DISPPARAMS, IDispatch,
            },
            LibraryLoader::{GetProcAddress, LoadLibraryW},
            Variant::{
                VAR_CHANGE_FLAGS, VARENUM, VARIANT, VARIANT_0_0, VT_BOOL, VT_BSTR, VT_BYREF, VT_I4,
                VT_I8, VT_VARIANT, VariantChangeType, VariantClear,
            },
        },
    },
    core::{GUID, HSTRING, PCSTR, PCWSTR},
};

use std::mem::ManuallyDrop;

pub trait VariantExt {
    fn by_ref(var_val: *mut VARIANT) -> VARIANT;
    fn to_i32(&self) -> windows::core::Result<i32>;
    fn to_i64(&self) -> windows::core::Result<i64>;
    fn to_string(&self) -> windows::core::Result<String>;
    fn to_bool(&self) -> windows::core::Result<bool>;
    fn from_str(s: &str) -> VARIANT;
}

impl VariantExt for VARIANT {
    fn by_ref(var_val: *mut VARIANT) -> VARIANT {
        let mut variant = VARIANT::default();
        let mut v00 = VARIANT_0_0 {
            vt: VARENUM(VT_BYREF.0 | VT_VARIANT.0),
            ..Default::default()
        };
        v00.Anonymous.pvarVal = var_val;
        variant.Anonymous.Anonymous = ManuallyDrop::new(v00);
        variant
    }
    fn to_i32(&self) -> windows::core::Result<i32> {
        unsafe {
            let mut new = VARIANT::default();
            VariantChangeType(&mut new, self, VAR_CHANGE_FLAGS(0), VT_I4)?;
            let v00 = &new.Anonymous.Anonymous;
            let n = v00.Anonymous.lVal;
            VariantClear(&mut new)?;
            Ok(n)
        }
    }
    fn to_i64(&self) -> windows::core::Result<i64> {
        unsafe {
            let mut new = VARIANT::default();
            VariantChangeType(&mut new, self, VAR_CHANGE_FLAGS(0), VT_I8)?;
            let v00 = &new.Anonymous.Anonymous;
            let n = v00.Anonymous.llVal;
            VariantClear(&mut new)?;
            Ok(n)
        }
    }
    fn to_string(&self) -> windows::core::Result<String> {
        unsafe {
            let mut new = VARIANT::default();
            VariantChangeType(&mut new, self, VAR_CHANGE_FLAGS(0), VT_BSTR)?;
            let v00 = &new.Anonymous.Anonymous;
            let str = v00.Anonymous.bstrVal.to_string();
            VariantClear(&mut new)?;
            Ok(str)
        }
    }
    fn to_bool(&self) -> windows::core::Result<bool> {
        unsafe {
            let mut new = VARIANT::default();
            VariantChangeType(&mut new, self, VAR_CHANGE_FLAGS(0), VT_BOOL)?;
            let v00 = &new.Anonymous.Anonymous;
            let b = v00.Anonymous.boolVal.as_bool();
            VariantClear(&mut new)?;
            Ok(b)
        }
    }
    fn from_str(s: &str) -> VARIANT {
        if s.is_empty() {
            VARIANT::default()
        } else {
            VARIANT::from(s)
        }
    }
}

// 对应 CARegJ 类
type FnSetDllPathW = unsafe extern "system" fn(PCWSTR, i32) -> i32;
static PFN_SET_DLL_PATH_W: OnceCell<Option<FnSetDllPathW>> = OnceCell::new();

fn set_dll_path(a_regj_path: String, ao_jia_path: String) -> i32 {
    let pfn = PFN_SET_DLL_PATH_W.get_or_init(|| unsafe {
        let a_regj_hstring = HSTRING::from(a_regj_path);
        let hmodule = LoadLibraryW(PCWSTR::from_raw(a_regj_hstring.as_ptr())).ok();
        hmodule.and_then(|h| {
            let proc_name = PCSTR::from_raw(r#"SetDllPathW "#.as_ptr());
            GetProcAddress(h, proc_name).map(|addr| std::mem::transmute(addr))
        })
    });

    if let Some(func) = pfn {
        unsafe {
            let ao_jia_hstring = HSTRING::from(ao_jia_path);
            func(PCWSTR::from_raw(ao_jia_hstring.as_ptr()), 0)
        }
    } else {
        0
    }
}

#[derive(Debug)]
pub struct AoJia {
    p_idispatch: Option<IDispatch>,
}

impl AoJia {
    const CLSID: GUID = GUID::from_values(
        0x4f27e588,
        0x5b1e,
        0x45b4,
        [0xad, 0x67, 0xe3, 0x2d, 0x45, 0xc4, 0xe9, 0xca],
    );

    fn new() -> windows::core::Result<Self> {
        unsafe {
            let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            if hr.is_err() {
                return Err(hr.into());
            }

            let idispatch: IDispatch = CoCreateInstance(&Self::CLSID, None, CLSCTX_INPROC_SERVER)?;

            Ok(Self {
                p_idispatch: Some(idispatch),
            })
        }
    }

    pub fn new_with_path(a_regj_path: String, ao_jia_path: String) -> windows::core::Result<Self> {
        set_dll_path(a_regj_path, ao_jia_path);
        Self::new()
    }

    fn invoke(
        &self,
        fun_name: &HSTRING,
        rgdispid: &mut i32,
        p_disp_params: &DISPPARAMS,
        p_var_result: &mut VARIANT,
    ) -> windows::core::Result<()> {
        unsafe {
            if *rgdispid == -1 {
                let names_ptr = PCWSTR::from_raw(fun_name.as_ptr());
                let names = [names_ptr];
                self.p_idispatch.as_ref().unwrap().GetIDsOfNames(
                    &GUID::default(),
                    names.as_ptr(),
                    1,
                    GetUserDefaultLCID(),
                    rgdispid,
                )?;
            }

            self.p_idispatch.as_ref().unwrap().Invoke(
                *rgdispid,
                &GUID::default(),
                GetUserDefaultLCID(),
                DISPATCH_METHOD,
                p_disp_params,
                Some(p_var_result),
                None,
                None,
            )
        }
    }
    #[allow(non_snake_case)]
    pub fn VerS(&self) -> windows::core::Result<String> {
        let fun_name = HSTRING::from("VerS");
        let mut disp_id = -1;
        let mut var_result = VARIANT::default();
        let disp_params = DISPPARAMS {
            rgvarg: ptr::null_mut(),
            rgdispidNamedArgs: ptr::null_mut(),
            cArgs: 0,
            cNamedArgs: 0,
        };

        self.invoke(&fun_name, &mut disp_id, &disp_params, &mut var_result)?;
        VariantExt::to_string(&var_result)
    }
    #[allow(non_snake_case)]
    pub fn SetPath(&self, Path: &str) -> windows::core::Result<i32> {
        let fun_name = HSTRING::from("SetPath");
        let mut disp_id = -1;
        let mut var_result = VARIANT::default();
        let mut args = [VARIANT::from(Path)];
        let disp_params = DISPPARAMS {
            rgvarg: args.as_mut_ptr(),
            rgdispidNamedArgs: ptr::null_mut(),
            cArgs: 1,
            cNamedArgs: 0,
        };

        self.invoke(&fun_name, &mut disp_id, &disp_params, &mut var_result)?;
        var_result.to_i32()
    }
    #[allow(non_snake_case)]
    pub fn SetErrorMsg(&self, Msg: i32) -> windows::core::Result<i32> {
        let fun_name = HSTRING::from("SetErrorMsg");
        let mut disp_id = -1;
        let mut var_result = VARIANT::default();
        let mut args = [VARIANT::from(Msg)];
        let disp_params = DISPPARAMS {
            rgvarg: args.as_mut_ptr(),
            rgdispidNamedArgs: ptr::null_mut(),
            cArgs: 1,
            cNamedArgs: 0,
        };

        self.invoke(&fun_name, &mut disp_id, &disp_params, &mut var_result)?;
        var_result.to_i32()
    }
    #[allow(non_snake_case)]
    pub fn SetThread(&self, TN: i32) -> windows::core::Result<i32> {
        let fun_name = HSTRING::from("SetThread");
        let mut disp_id = -1;
        let mut var_result = VARIANT::default();
        let mut args = [VARIANT::from(TN)];
        let disp_params = DISPPARAMS {
            rgvarg: args.as_mut_ptr(),
            rgdispidNamedArgs: ptr::null_mut(),
            cArgs: 1,
            cNamedArgs: 0,
        };

        self.invoke(&fun_name, &mut disp_id, &disp_params, &mut var_result)?;
        var_result.to_i32()
    }
    #[allow(non_snake_case)]
    pub fn GetModulePath(
        &self,
        PID: i32,
        Hwnd: i32,
        MN: &str,
        Type: i32,
    ) -> windows::core::Result<String> {
        let fun_name = HSTRING::from("GetModulePath");
        let mut disp_id = -1;
        let mut var_result = VARIANT::default();
        let mut args = [
            VARIANT::from(Type),
            VARIANT::from_str(MN),
            VARIANT::from(Hwnd),
            VARIANT::from(PID),
        ];
        let disp_params = DISPPARAMS {
            rgvarg: args.as_mut_ptr(),
            rgdispidNamedArgs: ptr::null_mut(),
            cArgs: 4,
            cNamedArgs: 0,
        };

        self.invoke(&fun_name, &mut disp_id, &disp_params, &mut var_result)?;
        VariantExt::to_string(&var_result)
    }
    #[allow(non_snake_case)]
    pub fn GetMachineCode(&self) -> windows::core::Result<String> {
        let fun_name = HSTRING::from("GetMachineCode");
        let mut disp_id = -1;
        let mut var_result = VARIANT::default();
        let disp_params = DISPPARAMS {
            rgvarg: ptr::null_mut(),
            rgdispidNamedArgs: ptr::null_mut(),
            cArgs: 0,
            cNamedArgs: 0,
        };

        self.invoke(&fun_name, &mut disp_id, &disp_params, &mut var_result)?;
        VariantExt::to_string(&var_result)
    }
    #[allow(non_snake_case)]
    pub fn GetOs(
        &self,
        SV: &mut String,
        SVN: &mut String,
        LVBN: &mut i32,
        SDir: &mut String,
        Type: i32,
    ) -> windows::core::Result<i32> {
        let fun_name = HSTRING::from("GetOs");
        let mut disp_id = -1;
        let mut var_result = VARIANT::default();

        let mut v = VARIANT::default();
        let mut vn = VARIANT::default();
        let mut vbn = VARIANT::default();
        let mut dir = VARIANT::default();

        let mut args = [
            VARIANT::from(Type),
            VARIANT::by_ref(&mut dir as *mut VARIANT), // dir
            VARIANT::by_ref(&mut vbn as *mut VARIANT), // vbn
            VARIANT::by_ref(&mut vn as *mut VARIANT),  // vn
            VARIANT::by_ref(&mut v as *mut VARIANT),   // v
        ];

        let disp_params = DISPPARAMS {
            rgvarg: args.as_mut_ptr(),
            rgdispidNamedArgs: ptr::null_mut(),
            cArgs: 5,
            cNamedArgs: 0,
        };

        self.invoke(&fun_name, &mut disp_id, &disp_params, &mut var_result)?;

        *SV = VariantExt::to_string(&v).unwrap_or_default();
        *SVN = VariantExt::to_string(&vn).unwrap_or_default();
        *LVBN = vbn.to_i32().unwrap_or(-1);
        *SDir = VariantExt::to_string(&dir).unwrap();
        var_result.to_i32()
    }
    #[allow(non_snake_case, clippy::too_many_arguments)]
    pub fn EnumWindow(
        &self,
        Parent: i32,
        ProName: &str,
        ProId: i32,
        Class: &str,
        Title: &str,
        Type: i32,
        Flag: i32,
        T: i32,
    ) -> windows::core::Result<String> {
        let fun_name = HSTRING::from("EnumWindow");
        let mut disp_id = -1;
        let mut var_result = VARIANT::default();
        let mut args = [
            VARIANT::from(T),
            VARIANT::from(Flag),
            VARIANT::from(Type),
            VARIANT::from_str(Title),
            VARIANT::from_str(Class),
            VARIANT::from(ProId),
            VARIANT::from_str(ProName),
            VARIANT::from(Parent),
        ];
        let disp_params = DISPPARAMS {
            rgvarg: args.as_mut_ptr(),
            rgdispidNamedArgs: ptr::null_mut(),
            cArgs: 8,
            cNamedArgs: 0,
        };

        self.invoke(&fun_name, &mut disp_id, &disp_params, &mut var_result)?;
        VariantExt::to_string(&var_result)
    }
    #[allow(non_snake_case, clippy::too_many_arguments)]
    pub fn FindWindow(
        &self,
        Parent: i32,
        ProName: &str,
        ProId: i32,
        Class: &str,
        Title: &str,
        Type: i32,
        T: i32,
    ) -> windows::core::Result<i32> {
        let fun_name = HSTRING::from("FindWindow");
        let mut disp_id = -1;
        let mut var_result = VARIANT::default();
        let mut args = [
            VARIANT::from(T),
            VARIANT::from(Type),
            VARIANT::from_str(Title),
            VARIANT::from_str(Class),
            VARIANT::from(ProId),
            VARIANT::from_str(ProName),
            VARIANT::from(Parent),
        ];
        let disp_params = DISPPARAMS {
            rgvarg: args.as_mut_ptr(),
            rgdispidNamedArgs: ptr::null_mut(),
            cArgs: 7,
            cNamedArgs: 0,
        };

        self.invoke(&fun_name, &mut disp_id, &disp_params, &mut var_result)?;
        var_result.to_i32()
    }
    #[allow(non_snake_case, clippy::too_many_arguments)]
    pub fn CreateWindows(
        &self,
        x: i32,
        y: i32,
        Width: i32,
        Height: i32,
        EWidth: i32,
        EHeight: i32,
        Type: i32,
    ) -> windows::core::Result<i32> {
        let fun_name = HSTRING::from("CreateWindows");
        let mut disp_id = -1;
        let mut var_result = VARIANT::default();
        let mut args = [
            VARIANT::from(Type),
            VARIANT::from(EHeight),
            VARIANT::from(EWidth),
            VARIANT::from(Height),
            VARIANT::from(Width),
            VARIANT::from(y),
            VARIANT::from(x),
        ];
        let disp_params = DISPPARAMS {
            rgvarg: args.as_mut_ptr(),
            rgdispidNamedArgs: ptr::null_mut(),
            cArgs: 7,
            cNamedArgs: 0,
        };

        self.invoke(&fun_name, &mut disp_id, &disp_params, &mut var_result)?;
        var_result.to_i32()
    }
    #[allow(non_snake_case)]
    pub fn GetRemoteProcAddress(
        &self,
        PID: i32,
        Hwnd: i32,
        MN: &str,
        Func: &str,
    ) -> windows::core::Result<i64> {
        let fun_name = HSTRING::from("GetRemoteProcAddress");
        let mut disp_id = -1;
        let mut var_result = VARIANT::default();
        let mut args = [
            VARIANT::from_str(Func),
            VARIANT::from_str(MN),
            VARIANT::from(Hwnd),
            VARIANT::from(PID),
        ];
        let disp_params = DISPPARAMS {
            rgvarg: args.as_mut_ptr(),
            rgdispidNamedArgs: ptr::null_mut(),
            cArgs: 4,
            cNamedArgs: 0,
        };

        self.invoke(&fun_name, &mut disp_id, &disp_params, &mut var_result)?;
        var_result.to_i64()
    }
    #[allow(non_snake_case)]
    pub fn KQHouTai(
        &self,
        Hwnd: i32,
        Screen: &str,
        Keyboard: &str,
        Mouse: &str,
        Flag: &str,
        Type: i32,
    ) -> windows::core::Result<i32> {
        let fun_name = HSTRING::from("KQHouTai");
        let mut disp_id = -1;
        let mut var_result = VARIANT::default();
        let mut args = [
            VARIANT::from(Type),
            VARIANT::from_str(Flag),
            VARIANT::from_str(Mouse),
            VARIANT::from_str(Keyboard),
            VARIANT::from_str(Screen),
            VARIANT::from(Hwnd),
        ];
        let disp_params = DISPPARAMS {
            rgvarg: args.as_mut_ptr(),
            rgdispidNamedArgs: ptr::null_mut(),
            cArgs: 6,
            cNamedArgs: 0,
        };

        self.invoke(&fun_name, &mut disp_id, &disp_params, &mut var_result)?;
        var_result.to_i32()
    }
    #[allow(non_snake_case)]
    pub fn GBHouTai(&self) -> windows::core::Result<i32> {
        let fun_name = HSTRING::from("GBHouTai");
        let mut disp_id = -1;
        let mut var_result = VARIANT::default();
        let disp_params = DISPPARAMS {
            rgvarg: ptr::null_mut(),
            rgdispidNamedArgs: ptr::null_mut(),
            cArgs: 0,
            cNamedArgs: 0,
        };

        self.invoke(&fun_name, &mut disp_id, &disp_params, &mut var_result)?;
        var_result.to_i32()
    }
    #[allow(non_snake_case)]
    pub fn GetCPU(&self, Type: &mut String, CPUID: &mut String) -> windows::core::Result<i32> {
        let fun_name = HSTRING::from("GetCPU");
        let mut disp_id = -1;
        let mut var_result = VARIANT::default();

        let mut ty = VARIANT::default();
        let mut id = VARIANT::default();

        let mut args = [
            VARIANT::by_ref(&mut id as *mut VARIANT),
            VARIANT::by_ref(&mut ty as *mut VARIANT),
        ];

        let disp_params = DISPPARAMS {
            rgvarg: args.as_mut_ptr(),
            rgdispidNamedArgs: ptr::null_mut(),
            cArgs: 2,
            cNamedArgs: 0,
        };

        self.invoke(&fun_name, &mut disp_id, &disp_params, &mut var_result)?;

        *Type = VariantExt::to_string(&ty).unwrap_or_default();
        *CPUID = VariantExt::to_string(&id).unwrap_or_default();
        var_result.to_i32()
    }
    #[allow(non_snake_case)]
    pub fn GetClientSize(
        &self,
        Hwnd: i32,
        Width: &mut i32,
        Height: &mut i32,
    ) -> windows::core::Result<i32> {
        let fun_name = HSTRING::from("GetClientSize");
        let mut disp_id = -1;
        let mut var_result = VARIANT::default();

        let mut w = VARIANT::default();
        let mut h = VARIANT::default();

        let mut args = [
            VARIANT::by_ref(&mut w as *mut VARIANT),
            VARIANT::by_ref(&mut h as *mut VARIANT),
            VARIANT::from(Hwnd),
        ];

        let disp_params = DISPPARAMS {
            rgvarg: args.as_mut_ptr(),
            rgdispidNamedArgs: ptr::null_mut(),
            cArgs: 3,
            cNamedArgs: 0,
        };

        self.invoke(&fun_name, &mut disp_id, &disp_params, &mut var_result)?;

        *Width = w.to_i32().unwrap_or(-1);
        *Height = h.to_i32().unwrap_or(-1);
        var_result.to_i32()
    }
    #[allow(non_snake_case)]
    pub fn GetWindowSize(
        &self,
        Hwnd: i32,
        Width: &mut i32,
        Height: &mut i32,
    ) -> windows::core::Result<i32> {
        let fun_name = HSTRING::from("GetWindowSize");
        let mut disp_id = -1;
        let mut var_result = VARIANT::default();

        let mut w = VARIANT::default();
        let mut h = VARIANT::default();

        let mut args = [
            VARIANT::by_ref(&mut h as *mut VARIANT),
            VARIANT::by_ref(&mut w as *mut VARIANT),
            VARIANT::from(Hwnd),
        ];

        let disp_params = DISPPARAMS {
            rgvarg: args.as_mut_ptr(),
            rgdispidNamedArgs: ptr::null_mut(),
            cArgs: 3,
            cNamedArgs: 0,
        };

        self.invoke(&fun_name, &mut disp_id, &disp_params, &mut var_result)?;

        *Width = w.to_i32().unwrap_or(-1);
        *Height = h.to_i32().unwrap_or(-1);
        var_result.to_i32()
    }
    #[allow(non_snake_case, clippy::too_many_arguments)]
    pub fn FindPic(
        &self,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        PicName: &str,
        ColorP: &str,
        Sim: f64,
        Dir: i32,
        Type: i32,
        Pic: &mut String,
        x: &mut i32,
        y: &mut i32,
    ) -> windows::core::Result<i32> {
        let fun_name = HSTRING::from("FindPic");
        let mut disp_id = -1;
        let mut var_result = VARIANT::default();

        // 创建返回值的变量
        let mut vx = VARIANT::default();
        let mut vy = VARIANT::default();
        let mut vpic = VARIANT::default();

        // 按照COM调用约定，参数顺序是反向的
        let mut args = [
            VARIANT::by_ref(&mut vy as *mut VARIANT),
            VARIANT::by_ref(&mut vx as *mut VARIANT),
            VARIANT::by_ref(&mut vpic as *mut VARIANT),
            VARIANT::from(Type),
            VARIANT::from(Dir),
            VARIANT::from(Sim),
            VARIANT::from_str(ColorP),
            VARIANT::from_str(PicName),
            VARIANT::from(y2),
            VARIANT::from(x2),
            VARIANT::from(y1),
            VARIANT::from(x1),
        ];

        let disp_params = DISPPARAMS {
            rgvarg: args.as_mut_ptr(),
            rgdispidNamedArgs: ptr::null_mut(),
            cArgs: 12,
            cNamedArgs: 0,
        };

        self.invoke(&fun_name, &mut disp_id, &disp_params, &mut var_result)?;

        // 获取返回值
        *Pic = VariantExt::to_string(&vpic).unwrap_or_default();
        *x = vx.to_i32().unwrap_or(-1);
        *y = vy.to_i32().unwrap_or(-1);

        var_result.to_i32()
    }
    #[allow(non_snake_case, clippy::too_many_arguments)]
    pub fn FindPicEx(
        &self,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        PicName: &str,
        ColorP: &str,
        Sim: f64,
        Dir: i32,
        Type: i32,
        TypeT: i32,
    ) -> windows::core::Result<String> {
        let fun_name = HSTRING::from("FindPicEx");
        let mut disp_id = -1;
        let mut var_result = VARIANT::default();

        // 按照COM调用约定，参数顺序是反向的
        let mut args = [
            VARIANT::from(TypeT),
            VARIANT::from(Type),
            VARIANT::from(Dir),
            VARIANT::from(Sim),
            VARIANT::from(ColorP),
            VARIANT::from(PicName),
            VARIANT::from(y2),
            VARIANT::from(x2),
            VARIANT::from(y1),
            VARIANT::from(x1),
        ];

        let disp_params = DISPPARAMS {
            rgvarg: args.as_mut_ptr(),
            rgdispidNamedArgs: ptr::null_mut(),
            cArgs: 10,
            cNamedArgs: 0,
        };

        self.invoke(&fun_name, &mut disp_id, &disp_params, &mut var_result)?;
        VariantExt::to_string(&var_result)
    }
    #[allow(non_snake_case)]
    pub fn ClientToScreen(
        &self,
        Hwnd: i32,
        x: &mut i32,
        y: &mut i32,
    ) -> windows::core::Result<i32> {
        let fun_name = HSTRING::from("ClientToScreen");
        let mut disp_id = -1;
        let mut var_result = VARIANT::default();

        let mut vx = VARIANT::default();
        let mut vy = VARIANT::default();

        let mut args = [
            VARIANT::by_ref(&mut vy as *mut VARIANT),
            VARIANT::by_ref(&mut vx as *mut VARIANT),
            VARIANT::from(Hwnd),
        ];

        let disp_params = DISPPARAMS {
            rgvarg: args.as_mut_ptr(),
            rgdispidNamedArgs: ptr::null_mut(),
            cArgs: 3,
            cNamedArgs: 0,
        };

        self.invoke(&fun_name, &mut disp_id, &disp_params, &mut var_result)?;

        *x = vx.to_i32().unwrap_or(-1);
        *y = vy.to_i32().unwrap_or(-1);

        var_result.to_i32()
    }
    #[allow(non_snake_case)]
    pub fn ClientOrScreen(
        &self,
        Hwnd: i32,
        xz: i32,
        yz: i32,
        x: &mut i32,
        y: &mut i32,
        Type: i32,
    ) -> windows::core::Result<i32> {
        let fun_name = HSTRING::from("ClientOrScreen");
        let mut disp_id = -1;
        let mut var_result = VARIANT::default();

        let mut vx = VARIANT::default();
        let mut vy = VARIANT::default();

        let mut args = [
            VARIANT::from(Type),
            VARIANT::by_ref(&mut vy as *mut VARIANT),
            VARIANT::by_ref(&mut vx as *mut VARIANT),
            VARIANT::from(yz),
            VARIANT::from(xz),
            VARIANT::from(Hwnd),
        ];

        let disp_params = DISPPARAMS {
            rgvarg: args.as_mut_ptr(),
            rgdispidNamedArgs: ptr::null_mut(),
            cArgs: 6,
            cNamedArgs: 0,
        };

        self.invoke(&fun_name, &mut disp_id, &disp_params, &mut var_result)?;

        *x = vx.to_i32().unwrap_or(-1);
        *y = vy.to_i32().unwrap_or(-1);

        var_result.to_i32()
    }
    #[allow(non_snake_case)]
    pub fn CompressFile(
        &self,
        SF: &str,
        DF: &str,
        Type: i32,
        Level: i32,
    ) -> windows::core::Result<i32> {
        let fun_name = HSTRING::from("CompressFile");
        let mut disp_id = -1;
        let mut var_result = VARIANT::default();
        let mut args = [
            VARIANT::from(Level),
            VARIANT::from(Type),
            VARIANT::from(DF),
            VARIANT::from(SF),
        ];
        let disp_params = DISPPARAMS {
            rgvarg: args.as_mut_ptr(),
            rgdispidNamedArgs: ptr::null_mut(),
            cArgs: 4,
            cNamedArgs: 0,
        };

        self.invoke(&fun_name, &mut disp_id, &disp_params, &mut var_result)?;
        var_result.to_i32()
    }

    #[allow(non_snake_case)]
    pub fn UnCompressFile(&self, SF: &str, DF: &str, Type: i32) -> windows::core::Result<i32> {
        let fun_name = HSTRING::from("UnCompressFile");
        let mut disp_id = -1;
        let mut var_result = VARIANT::default();
        let mut args = [VARIANT::from(Type), VARIANT::from(DF), VARIANT::from(SF)];
        let disp_params = DISPPARAMS {
            rgvarg: args.as_mut_ptr(),
            rgdispidNamedArgs: ptr::null_mut(),
            cArgs: 3,
            cNamedArgs: 0,
        };

        self.invoke(&fun_name, &mut disp_id, &disp_params, &mut var_result)?;
        var_result.to_i32()
    }
    #[allow(non_snake_case, clippy::too_many_arguments)]
    pub fn SetFont(
        &self,
        Hwnd: i32,
        Name: &str,
        Size: i32,
        Weight: i32,
        Italic: i32,
        Underline: i32,
        StrikeOut: i32,
    ) -> windows::core::Result<i32> {
        let fun_name = HSTRING::from("SetFont");
        let mut disp_id = -1;
        let mut var_result = VARIANT::default();
        let mut args = [
            VARIANT::from(StrikeOut),
            VARIANT::from(Underline),
            VARIANT::from(Italic),
            VARIANT::from(Weight),
            VARIANT::from(Size),
            VARIANT::from_str(Name),
            VARIANT::from(Hwnd),
        ];
        let disp_params = DISPPARAMS {
            rgvarg: args.as_mut_ptr(),
            rgdispidNamedArgs: ptr::null_mut(),
            cArgs: 7,
            cNamedArgs: 0,
        };

        self.invoke(&fun_name, &mut disp_id, &disp_params, &mut var_result)?;
        var_result.to_i32()
    }
    #[allow(non_snake_case, clippy::too_many_arguments)]
    pub fn SetTextD(
        &self,
        Hwnd: i32,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        Row: i32,
        Dir: i32,
    ) -> windows::core::Result<i32> {
        let fun_name = HSTRING::from("SetTextD");
        let mut disp_id = -1;
        let mut var_result = VARIANT::default();

        let mut args = [
            VARIANT::from(Dir),
            VARIANT::from(Row),
            VARIANT::from(y2),
            VARIANT::from(x2),
            VARIANT::from(y1),
            VARIANT::from(x1),
            VARIANT::from(Hwnd),
        ];

        let disp_params = DISPPARAMS {
            rgvarg: args.as_mut_ptr(),
            rgdispidNamedArgs: ptr::null_mut(),
            cArgs: 7,
            cNamedArgs: 0,
        };

        self.invoke(&fun_name, &mut disp_id, &disp_params, &mut var_result)?;
        var_result.to_i32()
    }
    #[allow(non_snake_case)]
    pub fn DrawTextD(
        &self,
        Hwnd: i32,
        Text: &str,
        Color: &str,
        BkColor: &str,
    ) -> windows::core::Result<i32> {
        let fun_name = HSTRING::from("DrawTextD");
        let mut disp_id = -1;
        let mut var_result = VARIANT::default();
        let mut args = [
            VARIANT::from_str(BkColor),
            VARIANT::from_str(Color),
            VARIANT::from_str(Text),
            VARIANT::from(Hwnd),
        ];
        let disp_params = DISPPARAMS {
            rgvarg: args.as_mut_ptr(),
            rgdispidNamedArgs: ptr::null_mut(),
            cArgs: 4,
            cNamedArgs: 0,
        };

        self.invoke(&fun_name, &mut disp_id, &disp_params, &mut var_result)?;
        var_result.to_i32()
    }
    #[allow(non_snake_case)]
    pub fn LeftClick(&self) -> windows::core::Result<i32> {
        let fun_name = HSTRING::from("LeftClick");
        let mut disp_id = -1;
        let mut var_result = VARIANT::default();
        let disp_params = DISPPARAMS {
            rgvarg: ptr::null_mut(),
            rgdispidNamedArgs: ptr::null_mut(),
            cArgs: 0,
            cNamedArgs: 0,
        };

        self.invoke(&fun_name, &mut disp_id, &disp_params, &mut var_result)?;
        var_result.to_i32()
    }
    #[allow(non_snake_case)]
    pub fn LeftDown(&self) -> windows::core::Result<i32> {
        let fun_name = HSTRING::from("LeftDown");
        let mut disp_id = -1;
        let mut var_result = VARIANT::default();
        let disp_params = DISPPARAMS {
            rgvarg: ptr::null_mut(),
            rgdispidNamedArgs: ptr::null_mut(),
            cArgs: 0,
            cNamedArgs: 0,
        };

        self.invoke(&fun_name, &mut disp_id, &disp_params, &mut var_result)?;
        var_result.to_i32()
    }
    #[allow(non_snake_case)]
    pub fn LeftUp(&self) -> windows::core::Result<i32> {
        let fun_name = HSTRING::from("LeftUp");
        let mut disp_id = -1;
        let mut var_result = VARIANT::default();
        let disp_params = DISPPARAMS {
            rgvarg: ptr::null_mut(),
            rgdispidNamedArgs: ptr::null_mut(),
            cArgs: 0,
            cNamedArgs: 0,
        };

        self.invoke(&fun_name, &mut disp_id, &disp_params, &mut var_result)?;
        var_result.to_i32()
    }
    #[allow(non_snake_case)]
    pub fn MoveTo(&self, x: i32, y: i32) -> windows::core::Result<i32> {
        let fun_name = HSTRING::from("MoveTo");
        let mut disp_id = -1;
        let mut var_result = VARIANT::default();
        let mut args = [VARIANT::from(y), VARIANT::from(x)];
        let disp_params = DISPPARAMS {
            rgvarg: args.as_mut_ptr(),
            rgdispidNamedArgs: ptr::null_mut(),
            cArgs: 2,
            cNamedArgs: 0,
        };

        self.invoke(&fun_name, &mut disp_id, &disp_params, &mut var_result)?;
        var_result.to_i32()
    }
    #[allow(non_snake_case)]
    pub fn WheelDown(&self) -> windows::core::Result<i32> {
        let fun_name = HSTRING::from("WheelDown");
        let mut disp_id = -1;
        let mut var_result = VARIANT::default();
        let disp_params = DISPPARAMS {
            rgvarg: ptr::null_mut(),
            rgdispidNamedArgs: ptr::null_mut(),
            cArgs: 0,
            cNamedArgs: 0,
        };

        self.invoke(&fun_name, &mut disp_id, &disp_params, &mut var_result)?;
        var_result.to_i32()
    }
    #[allow(non_snake_case)]
    pub fn YanShi(&self, RMin: i32, RMax: i32) -> windows::core::Result<i32> {
        let fun_name = HSTRING::from("YanShi");
        let mut disp_id = -1;
        let mut var_result = VARIANT::default();
        let mut args = [VARIANT::from(RMax), VARIANT::from(RMin)];
        let disp_params = DISPPARAMS {
            rgvarg: args.as_mut_ptr(),
            rgdispidNamedArgs: ptr::null_mut(),
            cArgs: 2,
            cNamedArgs: 0,
        };

        self.invoke(&fun_name, &mut disp_id, &disp_params, &mut var_result)?;
        var_result.to_i32()
    }
    #[allow(non_snake_case)]
    pub fn GetMousePos(&self, x: &mut i32, y: &mut i32, Type: i32) -> windows::core::Result<i32> {
        let fun_name = HSTRING::from("GetMousePos");
        let mut disp_id = -1;
        let mut var_result = VARIANT::default();

        let mut vx = VARIANT::default();
        let mut vy = VARIANT::default();

        let mut args = [
            VARIANT::from(Type),
            VARIANT::by_ref(&mut vy as *mut VARIANT),
            VARIANT::by_ref(&mut vx as *mut VARIANT),
        ];

        let disp_params = DISPPARAMS {
            rgvarg: args.as_mut_ptr(),
            rgdispidNamedArgs: ptr::null_mut(),
            cArgs: 3,
            cNamedArgs: 0,
        };

        self.invoke(&fun_name, &mut disp_id, &disp_params, &mut var_result)?;

        *x = vx.to_i32().unwrap_or(-1);
        *y = vy.to_i32().unwrap_or(-1);

        var_result.to_i32()
    }
    #[allow(non_snake_case)]
    pub fn LoadDict(&self, DNum: i32, DName: &str) -> windows::core::Result<i32> {
        let fun_name = HSTRING::from("LoadDict");
        let mut disp_id = -1;
        let mut var_result = VARIANT::default();
        let mut args = [VARIANT::from(DName), VARIANT::from(DNum)];
        let disp_params = DISPPARAMS {
            rgvarg: args.as_mut_ptr(),
            rgdispidNamedArgs: ptr::null_mut(),
            cArgs: 2,
            cNamedArgs: 0,
        };

        self.invoke(&fun_name, &mut disp_id, &disp_params, &mut var_result)?;
        var_result.to_i32()
    }
    #[allow(non_snake_case)]
    pub fn SetDict(&self, DNum: i32) -> windows::core::Result<i32> {
        let fun_name = HSTRING::from("SetDict");
        let mut disp_id = -1;
        let mut var_result = VARIANT::default();
        let mut args = [VARIANT::from(DNum)];
        let disp_params = DISPPARAMS {
            rgvarg: args.as_mut_ptr(),
            rgdispidNamedArgs: ptr::null_mut(),
            cArgs: 1,
            cNamedArgs: 0,
        };

        self.invoke(&fun_name, &mut disp_id, &disp_params, &mut var_result)?;
        var_result.to_i32()
    }
    #[allow(non_snake_case, clippy::too_many_arguments)]
    pub fn Ocr(
        &self,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        Str: &str,
        Color: &str,
        Sim: f64,
        TypeC: i32,
        TypeD: i32,
        TypeR: i32,
        TypeT: i32,
        HLine: &str,
        PicName: &str,
    ) -> windows::core::Result<String> {
        let fun_name = HSTRING::from("Ocr");
        let mut disp_id = -1;
        let mut var_result = VARIANT::default();

        let mut args = [
            VARIANT::from_str(PicName),
            VARIANT::from_str(HLine),
            VARIANT::from(TypeT),
            VARIANT::from(TypeR),
            VARIANT::from(TypeD),
            VARIANT::from(TypeC),
            VARIANT::from(Sim),
            VARIANT::from_str(Color),
            VARIANT::from_str(Str),
            VARIANT::from(y2),
            VARIANT::from(x2),
            VARIANT::from(y1),
            VARIANT::from(x1),
        ];

        let disp_params = DISPPARAMS {
            rgvarg: args.as_mut_ptr(),
            rgdispidNamedArgs: ptr::null_mut(),
            cArgs: 13,
            cNamedArgs: 0,
        };

        self.invoke(&fun_name, &mut disp_id, &disp_params, &mut var_result)?;
        VariantExt::to_string(&var_result)
    }
}

impl Drop for AoJia {
    fn drop(&mut self) {
        unsafe {
            // IDispatch implements Drop which will call Release internally
            // Just let it drop automatically
            self.p_idispatch.take();
            CoUninitialize();
        }
    }
}
