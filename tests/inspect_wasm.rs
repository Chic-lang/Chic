use chic::runtime::wasm_executor::instructions::Instruction;
use chic::runtime::wasm_executor::parser::parse_module;
use std::collections::HashMap;

fn index_exports(exports: &HashMap<String, u32>) -> HashMap<u32, Vec<String>> {
    let mut map: HashMap<u32, Vec<String>> = HashMap::new();
    for (name, &index) in exports {
        map.entry(index).or_default().push(name.clone());
    }
    map
}

#[test]
fn inspect_litmus_calls() {
    let path = std::env::var("INSPECT_WASM_PATH")
        .unwrap_or_else(|_| "tests/concurrency/litmus/fixtures.wasm".to_string());
    println!("inspecting wasm at {path}");
    let bytes = std::fs::read(&path).expect("read fixtures.wasm");
    let module = parse_module(&bytes).expect("parse module");
    let export_names = index_exports(&module.exports);
    let import_count = module.imports.len() as u32;

    println!("memory min pages: {:?}", module.memory_min_pages);
    println!("globals:");
    for (idx, global) in module.globals.iter().enumerate() {
        println!(
            "  {idx}: ty={:?} mutable={} init={:?}",
            global.ty, global.mutable, global.initial
        );
    }
    let debug_funcs: [u32; 10] = [95, 101, 102, 111, 117, 189, 240, 245, 1471, 1576];
    for func in debug_funcs {
        println!(
            "exports for func {}: {:?}",
            func,
            export_names.get(&func).cloned().unwrap_or_default()
        );
    }
    let extra_funcs: [u32; 2] = [1587, 1465];
    for func in extra_funcs {
        println!(
            "exports for func {}: {:?}",
            func,
            export_names.get(&func).cloned().unwrap_or_default()
        );
    }

    println!("imports:");
    for (idx, import) in module.imports.iter().enumerate() {
        println!("  {} => {}::{}", idx, import.module, import.name);
    }

    println!("types:");
    for (idx, ty) in module.types.iter().enumerate() {
        println!("  {idx}: params={:?} results={:?}", ty.params, ty.results);
    }

    println!("exports matching `ThreadFunction`:");
    let mut thread_exports = Vec::new();
    for (name, index) in module
        .exports
        .iter()
        .filter(|(name, _)| name.contains("ThreadFunction"))
    {
        println!("  {name} => {index}");
        thread_exports.push(*index);
    }
    thread_exports.sort();
    thread_exports.dedup();

    let mut factory_exports = Vec::new();
    println!("exports matching `ThreadStartFactory::Function`:");
    for (name, index) in module
        .exports
        .iter()
        .filter(|(name, _)| name.contains("ThreadStartFactory::Function"))
    {
        println!("  {name} => {index}");
        factory_exports.push(*index);
    }
    factory_exports.sort();
    factory_exports.dedup();

    for (idx, table) in module.tables.iter().enumerate() {
        let populated: Vec<_> = table
            .elements
            .iter()
            .enumerate()
            .filter_map(|(slot, value)| value.map(|func| (slot, func)))
            .collect();
        println!(
            "table {idx}: min={} max={:?} populated_slots={:?}",
            table.min, table.max, populated
        );
    }

    let interesting: [u32; 23] = [
        3, 16, 17, 83, 84, 94, 95, 96, 101, 102, 112, 189, 239, 240, 245, 1466, 1488, 1528, 1532,
        1576, 1587, 1592, 1595,
    ];
    let dump_full: [u32; 13] = [
        83, 112, 189, 239, 240, 245, 1488, 1528, 1532, 1576, 1587, 1592, 1595,
    ];
    for func_index in interesting {
        let exports = export_names.get(&func_index).cloned().unwrap_or_default();
        println!("func {func_index} exports={exports:?}");
        if func_index < import_count {
            println!("  (import)");
            continue;
        }
        let slot = func_index - import_count;
        let function = module
            .functions
            .get(slot as usize)
            .expect("function present in module");
        if func_index == 84 {
            println!("instructions for func 84:");
            for (pc, instr) in function.code.iter().enumerate() {
                println!("  pc={pc} {:?}", instr);
            }
        }
        if dump_full.contains(&func_index) {
            println!("full instructions for func {func_index} exports={exports:?}:");
            for (pc, instr) in function.code.iter().enumerate() {
                println!("  pc={pc} {:?}", instr);
            }
        }
        for (pc, instr) in function.code.iter().enumerate() {
            match instr {
                Instruction::Call { func } => {
                    let targets = export_names.get(func).cloned().unwrap_or_default();
                    let label = if (*func as usize) < module.imports.len() {
                        let import = &module.imports[*func as usize];
                        format!("import {}::{}", import.module, import.name)
                    } else {
                        "local".to_string()
                    };
                    println!("  pc={pc} call {func} ({label}) exports={targets:?}");
                }
                Instruction::CallIndirect { type_index, .. } => {
                    println!("  pc={pc} call_indirect type={type_index}");
                }
                _ => {}
            }
        }
    }

    let import_count = module.imports.len() as u32;
    for (idx, function) in module.functions.iter().enumerate() {
        let func_index = import_count + idx as u32;
        let exports = export_names.get(&func_index).cloned().unwrap_or_default();
        let mut printed_listing = false;
        for (pc, instr) in function.code.iter().enumerate() {
            if let Instruction::CallIndirect {
                type_index,
                table_index,
            } = instr
            {
                if !printed_listing {
                    println!("instructions for func {func_index} exports={exports:?}:");
                    for (idx, instr) in function.code.iter().enumerate() {
                        println!("  pc={idx} {:?}", instr);
                    }
                    printed_listing = true;
                }
                println!(
                    "call_indirect func_index={} exports={:?} pc={} type={} table={}",
                    func_index, exports, pc, type_index, table_index
                );
            }
        }
        if !printed_listing
            && (thread_exports.binary_search(&func_index).is_ok()
                || factory_exports.binary_search(&func_index).is_ok())
        {
            println!("instructions for func {func_index} exports={exports:?}:");
            for (idx, instr) in function.code.iter().enumerate() {
                println!("  pc={idx} {:?}", instr);
            }
        }
    }

    let target_types = [
        0x0d90ca9dc149156b_u64,
        0x8f5243907f6ea195_u64,
        0xfa752b38982cda52_u64,
    ];
    for target_type in target_types {
        let mut found = false;
        for record in &module.type_metadata {
            if record.type_id == target_type {
                println!(
                    "type metadata for 0x{target_type:016x}: size={} align={}",
                    record.size, record.align
                );
                found = true;
            }
        }
        if !found {
            println!("type metadata for 0x{target_type:016x} missing");
        }
    }
    let zero_types: Vec<_> = module
        .type_metadata
        .iter()
        .filter(|record| record.size == 0 || record.align == 0)
        .map(|record| {
            format!(
                "0x{:016x} (size={}, align={})",
                record.type_id, record.size, record.align
            )
        })
        .collect();
    if !zero_types.is_empty() {
        println!("zero-sized metadata entries: {}", zero_types.join(", "));
    }

    // Dump callers of ThreadStartFactory::Function (1471) to inspect argument setup.
    let factory_index: u32 = 1471;
    for (idx, function) in module.functions.iter().enumerate() {
        let func_index = import_count + idx as u32;
        if !function
            .code
            .iter()
            .any(|instr| matches!(instr, Instruction::Call { func } if *func == factory_index))
        {
            continue;
        }
        let exports = export_names.get(&func_index).cloned().unwrap_or_default();
        println!(
            "factory caller func_index={} exports={:?}:",
            func_index, exports
        );
        for (pc, instr) in function.code.iter().enumerate() {
            println!("  pc={pc} {:?}", instr);
        }
    }
}
