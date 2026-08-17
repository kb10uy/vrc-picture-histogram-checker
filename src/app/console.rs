use windows::{
    Win32::{
        Foundation::GENERIC_WRITE,
        Storage::FileSystem::{
            CreateFileW, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_WRITE, OPEN_EXISTING,
        },
        System::Console::{
            ATTACH_PARENT_PROCESS, AttachConsole, GetStdHandle, STD_ERROR_HANDLE, STD_HANDLE,
            STD_OUTPUT_HANDLE, SetStdHandle,
        },
    },
    core::w,
};

pub fn attach_to_parent() {
    if unsafe { AttachConsole(ATTACH_PARENT_PROCESS) }.is_err() {
        return;
    }
    open_console(STD_OUTPUT_HANDLE);
    open_console(STD_ERROR_HANDLE);
}

fn open_console(id: STD_HANDLE) {
    if unsafe { GetStdHandle(id) }.is_ok_and(|existing| !existing.is_invalid()) {
        return;
    }

    let opened = unsafe {
        CreateFileW(
            w!("CONOUT$"),
            GENERIC_WRITE.0,
            FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_FLAGS_AND_ATTRIBUTES::default(),
            None,
        )
    };
    if let Ok(handle) = opened {
        let _ = unsafe { SetStdHandle(id, handle) };
    }
}
