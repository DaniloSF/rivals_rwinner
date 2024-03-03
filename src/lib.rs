mod config;
pub mod consts;

use crate::consts::{BASE_ADDRESS, PTR_OFFSET, SET_VAR};
use consts::PTR_BASE;
use retour::static_detour;
use tracing::info;

use std::io::Write;
use std::sync::OnceLock;
use std::{mem, net::TcpStream, sync::Mutex};
use windows::{
    core::PCSTR,
    Win32::{Foundation::HMODULE, System::LibraryLoader::GetModuleHandleA},
};

type SetVarFn = extern "thiscall" fn(*mut YYVar, *mut YYVar);

static_detour! {
    static SetVarHook: extern "thiscall" fn(*mut YYVar, *mut YYVar);
}

static DATA_STREAM: OnceLock<TcpStream> = OnceLock::new();
static DEBUG_STREAM: OnceLock<TcpStream> = OnceLock::new();

#[ctor::ctor]
fn main() {
    DATA_STREAM.get_or_init(|| {
        let data_stream = TcpStream::connect(config::get_data_address()).unwrap();
        data_stream
    });

    let debug_stream = DEBUG_STREAM.get_or_init(|| {
        TcpStream::connect(config::get_debug_address()).unwrap()
    });

    tracing_subscriber::fmt()
        .with_writer(Mutex::new(debug_stream))
        .init();

    if let Err(e) = unsafe { cmon_work() } {
        clean_up_with_error(e)
    }
}

#[ctor::dtor]
fn dtor() {
    unsafe {
        SetVarHook.disable().unwrap();
    }
}

unsafe fn cmon_work() -> color_eyre::Result<()> {
    BASE_ADDRESS = unsafe { get_base_address() };

    let set_var_script_address: usize = BASE_ADDRESS + SET_VAR;

    let o_set_var: SetVarFn = mem::transmute(set_var_script_address);

    SetVarHook
        .initialize(o_set_var, gml_script_set_var_hooked)
        .unwrap();
    SetVarHook.enable().unwrap();

    Ok(())
}

#[no_mangle]
fn gml_script_set_var_hooked(this: *mut YYVar, a2: *mut YYVar) {
    unsafe {
        let player_won_ptr = get_player_won_ptr();
        if !player_won_ptr.is_null() && this == player_won_ptr {
            info!("Player won: {}", (*a2).value);

            DATA_STREAM
                .get()
                .unwrap()
                .write_all(&(*a2).value.to_ne_bytes())
                .unwrap();
        }

        SetVarHook.call(this, a2);
    }
}

#[repr(C)]
struct YYVar {
    value: f64,
    field_8: i32,
    field_c: i32,
}

fn get_player_won_ptr() -> *mut YYVar {
    unsafe {
        let base = (BASE_ADDRESS + PTR_BASE) as *mut u64;

        let jump_res = make_ptr_jump(base, &PTR_OFFSET);

        // Holds the pointer value obtained from the `jump_res` result.
        // If `jump_res` is `Ok`, it contains the valid pointer value.
        // If `jump_res` is `Err`, it returns a null mutable pointer.
        let ptr_jump: *mut YYVar = match jump_res {
            Ok(ptr) => ptr,
            Err(e) => {
                clean_up_with_error(e);
                std::ptr::null_mut()
            }
        };

        ptr_jump
    }
}

fn clean_up_with_error(e: color_eyre::eyre::Error) {
    clean_up();
    panic!("{}", e);
}

fn clean_up() {
    unsafe {
        SetVarHook.disable().unwrap();
    }
    DATA_STREAM
        .get()
        .unwrap()
        .shutdown(std::net::Shutdown::Both)
        .unwrap();
    DEBUG_STREAM
        .get()
        .unwrap()
        .shutdown(std::net::Shutdown::Both)
        .unwrap();
}

fn make_ptr_jump(ptr: *mut u64, offsets: &[isize]) -> color_eyre::Result<*mut YYVar> {
    let mut ptr_jump = ptr;

    for offset in offsets {
        unsafe {
            // ensure that the pointer is not null, if it is not the last offset then check if it is a valid pointer
            if ptr_jump.is_null() {
                return Err(color_eyre::Report::msg(format!(
                    "Null pointer in make_ptr_jump before offset {}",
                    offset,
                )));
            }
            // check if the pointer is in range of the process memory
            if ptr_jump.read() < (BASE_ADDRESS as u64) || ptr_jump.read() > 0x7FFFFFFF_FFFFFFFF {
                return Err(color_eyre::Report::msg(
                    "Invalid pointer in make_ptr_jump, out of range",
                ));
            }

            ptr_jump = (ptr_jump.read() as *mut u64).byte_offset(*offset);
        }
    }
    Ok(ptr_jump as *mut YYVar)
}

unsafe fn get_base_address() -> usize {
    let module_handle = GetModuleHandleA(PCSTR::null());
    module_handle.unwrap_or(HMODULE(0)).0 as usize
}
