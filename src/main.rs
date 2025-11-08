use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::cell::RefCell;
use rfd::FileDialog;

thread_local! {
    static COD_PATH: Rc<RefCell<Option<PathBuf>>> = Rc::new(RefCell::new(None));
}

fn main() {
    if let Some(default_path) = check_cod_default() {
        COD_PATH.with(|p| *p.borrow_mut() = Some(default_path.clone()));
        println!("Default COD path found: {:?}", default_path);
    } else {
        println!("COD default path not found");
    }

    pick_cod_directory();
}


fn check_cod_default() -> Option<PathBuf> {
    let default_path = Path::new(r"C:/Program Files (x86)/Steam/steamapps/common/Call of Duty");
    let cod_exe = default_path.join("cod.exe");

    if cod_exe.is_file() {
        Some(default_path.to_path_buf())
    } else {
        None
    }
}

fn pick_cod_directory() {
    if let Some(selected) = FileDialog::new().pick_folder() {
        let cod_exe = selected.join("cod.exe");
        if cod_exe.is_file() {
            COD_PATH.with(|p| *p.borrow_mut() = Some(selected.clone()));
            println!("COD path set to: {:?}", selected);
        } else {
            println!("cod.exe not found in selected folder: {:?}", selected);
        }
    } else {
        println!("No folder selected");
    }
}

pub fn get_cod_path() -> Option<PathBuf> {
    COD_PATH.with(|p| p.borrow().clone())
}
