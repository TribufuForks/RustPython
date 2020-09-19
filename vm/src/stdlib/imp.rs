use crate::import;
use crate::obj::objbytes::PyBytesRef;
use crate::obj::objcode::PyCode;
use crate::obj::objmodule::PyModuleRef;
use crate::obj::objstr;
use crate::obj::objstr::PyStringRef;
use crate::pyobject::{BorrowValue, ItemProtocol, PyObjectRef, PyResult};
use crate::vm::VirtualMachine;

#[cfg(feature = "threading")]
mod lock {
    use crate::pyobject::PyResult;
    use crate::stdlib::thread::RawRMutex;
    use crate::vm::VirtualMachine;

    pub(super) static IMP_LOCK: RawRMutex = RawRMutex::INIT;

    pub(super) fn imp_acquire_lock(_vm: &VirtualMachine) {
        IMP_LOCK.lock()
    }

    pub(super) fn imp_release_lock(vm: &VirtualMachine) -> PyResult<()> {
        if !IMP_LOCK.is_locked() {
            Err(vm.new_runtime_error("Global import lock not held".to_owned()))
        } else {
            unsafe { IMP_LOCK.unlock() };
            Ok(())
        }
    }

    pub(super) fn imp_lock_held(_vm: &VirtualMachine) -> bool {
        IMP_LOCK.is_locked()
    }
}

#[cfg(not(feature = "threading"))]
mod lock {
    use crate::vm::VirtualMachine;
    pub(super) fn imp_acquire_lock(_vm: &VirtualMachine) {}
    pub(super) fn imp_release_lock(_vm: &VirtualMachine) {}
    pub(super) fn imp_lock_held(_vm: &VirtualMachine) -> bool {
        false
    }
}

use lock::{imp_acquire_lock, imp_lock_held, imp_release_lock};

fn imp_extension_suffixes(vm: &VirtualMachine) -> PyResult {
    Ok(vm.ctx.new_list(vec![]))
}

fn imp_is_builtin(name: PyStringRef, vm: &VirtualMachine) -> bool {
    vm.state.stdlib_inits.contains_key(name.borrow_value())
}

fn imp_is_frozen(name: PyStringRef, vm: &VirtualMachine) -> bool {
    vm.state.frozen.contains_key(name.borrow_value())
}

fn imp_create_builtin(spec: PyObjectRef, vm: &VirtualMachine) -> PyResult {
    let sys_modules = vm.get_attribute(vm.sys_module.clone(), "modules").unwrap();
    let spec = vm.get_attribute(spec, "name")?;
    let name = objstr::borrow_value(&spec);

    if let Ok(module) = sys_modules.get_item(name, vm) {
        Ok(module)
    } else if let Some(make_module_func) = vm.state.stdlib_inits.get(name) {
        Ok(make_module_func(vm))
    } else {
        Ok(vm.ctx.none())
    }
}

fn imp_exec_builtin(_mod: PyModuleRef) -> i32 {
    // TOOD: Should we do something here?
    0
}

fn imp_get_frozen_object(name: PyStringRef, vm: &VirtualMachine) -> PyResult<PyCode> {
    let name = name.borrow_value();
    vm.state
        .frozen
        .get(name)
        .map(|frozen| {
            let mut frozen = frozen.code.clone();
            frozen.source_path = format!("frozen {}", name);
            PyCode::new(frozen)
        })
        .ok_or_else(|| vm.new_import_error(format!("No such frozen object named {}", name), name))
}

fn imp_init_frozen(name: PyStringRef, vm: &VirtualMachine) -> PyResult {
    import::import_frozen(vm, name.borrow_value())
}

fn imp_is_frozen_package(name: PyStringRef, vm: &VirtualMachine) -> PyResult<bool> {
    let name = name.borrow_value();
    vm.state
        .frozen
        .get(name)
        .map(|frozen| frozen.package)
        .ok_or_else(|| vm.new_import_error(format!("No such frozen object named {}", name), name))
}

fn imp_fix_co_filename(_code: PyObjectRef, _path: PyStringRef) {
    // TODO:
}

fn imp_source_hash(_key: u64, _source: PyBytesRef, vm: &VirtualMachine) -> PyResult {
    // TODO:
    Ok(vm.ctx.none())
}

pub fn make_module(vm: &VirtualMachine) -> PyObjectRef {
    let ctx = &vm.ctx;
    py_module!(vm, "_imp", {
        "extension_suffixes" => ctx.new_function(imp_extension_suffixes),
        "acquire_lock" => ctx.new_function(imp_acquire_lock),
        "release_lock" => ctx.new_function(imp_release_lock),
        "lock_held" => ctx.new_function(imp_lock_held),
        "is_builtin" => ctx.new_function(imp_is_builtin),
        "is_frozen" => ctx.new_function(imp_is_frozen),
        "create_builtin" => ctx.new_function(imp_create_builtin),
        "exec_builtin" => ctx.new_function(imp_exec_builtin),
        "get_frozen_object" => ctx.new_function(imp_get_frozen_object),
        "init_frozen" => ctx.new_function(imp_init_frozen),
        "is_frozen_package" => ctx.new_function(imp_is_frozen_package),
        "_fix_co_filename" => ctx.new_function(imp_fix_co_filename),
        "source_hash" => ctx.new_function(imp_source_hash)
    })
}
