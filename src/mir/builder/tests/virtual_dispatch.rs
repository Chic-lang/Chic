use super::common::RequireExt;
use super::*;

#[test]
fn class_vtables_record_slots_and_virtual_calls() {
    let source = r#"
namespace Dispatch;

public class Animal
{
    public virtual int Speak() { return 1; }
    public virtual int Chain() { return Speak() + 10; }
}

public class Dog : Animal
{
    public sealed override int Speak() { return 2; }
    public override int Chain() { return base.Chain() + 20; }
    public int SpeakAsBase() { return base.Speak(); }
}

public static class Tests
{
    public static int Invoke(Animal target) { return target.Speak(); }
}
"#;

    let parsed = parse_module(source).require("parse class hierarchy");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let module = lowering.module;
    assert!(
        module.class_vtables.len() >= 2,
        "expected Animal and Dog class tables"
    );
    let animal = module
        .class_vtables
        .iter()
        .find(|table| table.type_name == "Dispatch::Animal")
        .expect("Animal vtable");
    assert_eq!(animal.slots.len(), 2, "Animal should expose Speak + Chain");
    assert_eq!(animal.slots[0].member, "Speak");
    assert_eq!(animal.slots[0].symbol, "Dispatch::Animal::Speak");
    assert_eq!(animal.slots[1].member, "Chain");
    assert_eq!(animal.slots[1].symbol, "Dispatch::Animal::Chain");

    let dog = module
        .class_vtables
        .iter()
        .find(|table| table.type_name == "Dispatch::Dog")
        .expect("Dog vtable");
    assert_eq!(dog.slots.len(), 2, "Dog should reuse inherited slots");
    let speak_slot = dog
        .slots
        .iter()
        .find(|slot| slot.member == "Speak")
        .expect("speak slot");
    assert_eq!(speak_slot.slot_index, 0, "Speak should preserve slot index");
    assert_eq!(speak_slot.symbol, "Dispatch::Dog::Speak");
    let chain_slot = dog
        .slots
        .iter()
        .find(|slot| slot.member == "Chain")
        .expect("chain slot");
    assert_eq!(chain_slot.slot_index, 1);
    assert_eq!(chain_slot.symbol, "Dispatch::Dog::Chain");
}
