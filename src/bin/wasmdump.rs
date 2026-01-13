use chic::runtime::wasm_executor::instructions::Instruction;
use chic::runtime::wasm_executor::parser::parse_module;
use std::env;
use std::fs;

fn main() {
    let mut args = env::args().skip(1);
    let path = args.next().expect("path");
    let next = args.next().expect("index or --find");
    let mode_find = next == "--find";
    let mode_name = next == "--name";
    let mode_imports = next == "--imports";
    let mode_types = next == "--types";
    let mode_signature = next == "--sig";
    if next == "--exports" {
        let bytes = fs::read(&path).expect("read wasm");
        let module = parse_module(&bytes).expect("parse module");
        println!(
            "imports: {} functions: {} exports: {}",
            module.imports.len(),
            module.functions.len(),
            module.exports.len()
        );
        for (name, idx) in module.exports.iter() {
            println!("{idx}: {name}");
        }
        return;
    }
    if mode_imports {
        let bytes = fs::read(&path).expect("read wasm");
        let module = parse_module(&bytes).expect("parse module");
        for (idx, import) in module.imports.iter().enumerate() {
            println!(
                "{idx}: {}::{} (type {})",
                import.module, import.name, import.type_index
            );
        }
        return;
    }
    if mode_types {
        let bytes = fs::read(&path).expect("read wasm");
        let module = parse_module(&bytes).expect("parse module");
        println!("type metadata entries: {}", module.type_metadata.len());
        for entry in &module.type_metadata {
            println!(
                "type_id=0x{:016x} size={} align={} variance={:?}",
                entry.type_id, entry.size, entry.align, entry.variance
            );
        }
        return;
    }
    let func_index: u32 = if mode_find || mode_name || mode_signature {
        args.next().expect("index").parse().expect("index")
    } else {
        next.parse().expect("index")
    };
    let bytes = fs::read(&path).expect("read wasm");
    let module = parse_module(&bytes).expect("parse module");
    let import_count = module.imports.len() as u32;
    if mode_signature {
        let type_index = if func_index < import_count {
            module
                .imports
                .get(func_index as usize)
                .map(|import| import.type_index)
        } else {
            module
                .functions
                .get((func_index - import_count) as usize)
                .map(|func| func.type_index)
        }
        .expect("function index out of range");
        let signature = module
            .types
            .get(type_index as usize)
            .expect("missing signature entry");
        println!(
            "func {} type {} params={:?} results={:?}",
            func_index, type_index, signature.params, signature.results
        );
        return;
    }
    println!(
        "imports: {} functions: {}",
        module.imports.len(),
        module.functions.len()
    );
    if mode_name {
        if let Some((export, _)) = module
            .exports
            .iter()
            .find(|(_, index)| **index == func_index)
        {
            println!("exported as {}", export);
        }
        for default in &module.interface_defaults {
            if default.function_index == func_index {
                println!(
                    "interface default {}::{} implemented by {}",
                    default.interface, default.method, default.implementer
                );
            }
        }
        if (func_index as usize) < module.imports.len() {
            let import = &module.imports[func_index as usize];
            println!(
                "func {} is import {}::{}",
                func_index, import.module, import.name
            );
        } else {
            println!(
                "func {} is local index {}",
                func_index,
                func_index - import_count
            );
        }
        return;
    }
    if mode_find {
        for (idx, func) in module.functions.iter().enumerate() {
            let func_idx = import_count + idx as u32;
            if func
                .code
                .iter()
                .any(|instr| matches!(instr, Instruction::Call { func } if *func == func_index))
            {
                println!("call from func {}", func_idx);
            }
        }
        return;
    }
    if func_index < import_count {
        println!("func {} is import", func_index);
        return;
    }
    let local_index = func_index - import_count;
    let func = module
        .functions
        .get(local_index as usize)
        .expect("function");
    println!("type index: {} locals: {:?}", func.type_index, func.locals);
    for (pc, instr) in func.code.iter().enumerate() {
        println!("{}: {:?}", pc, instr);
    }
}
