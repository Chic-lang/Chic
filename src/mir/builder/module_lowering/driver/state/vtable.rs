use super::helpers::dispatch_participates;
use super::*;
use crate::mir::class_vtable_symbol_name;
use blake3::Hasher;

#[derive(Clone)]
pub(super) struct ClassVTablePlan {
    type_name: String,
    symbol: String,
    slots: Vec<ClassVTableSlotPlan>,
    lookup: HashMap<String, usize>,
}

#[derive(Clone)]
struct ClassVTableSlotPlan {
    member: String,
    accessor: Option<PropertyAccessorKind>,
    slot_index: u32,
    sealed: bool,
    function_symbol: Option<String>,
}

impl ClassVTablePlan {
    fn new(name: &str) -> Self {
        Self {
            type_name: name.to_string(),
            symbol: class_vtable_symbol_name(name),
            slots: Vec::new(),
            lookup: HashMap::new(),
        }
    }

    fn inherit_from(&mut self, base: &ClassVTablePlan) {
        for slot in &base.slots {
            let index = self.slots.len();
            self.lookup.insert(slot.member.clone(), index);
            self.slots.push(slot.clone());
        }
    }

    fn ensure_slot(
        &mut self,
        member: String,
        accessor: Option<PropertyAccessorKind>,
        sealed: bool,
    ) -> u32 {
        if let Some(&index) = self.lookup.get(&member) {
            if sealed {
                self.slots[index].sealed = true;
            }
            return self.slots[index].slot_index;
        }
        let slot_index = self.slots.len() as u32;
        let entry = ClassVTableSlotPlan {
            member: member.clone(),
            accessor,
            slot_index,
            sealed,
            function_symbol: None,
        };
        self.lookup.insert(member, self.slots.len());
        self.slots.push(entry);
        slot_index
    }

    fn slot_mut(&mut self, member: &str) -> Option<&mut ClassVTableSlotPlan> {
        let index = *self.lookup.get(member)?;
        self.slots.get_mut(index)
    }

    fn compute_version(&self) -> u64 {
        let mut hasher = Hasher::new();
        hasher.update(self.type_name.as_bytes());
        hasher.update(self.symbol.as_bytes());
        for slot in &self.slots {
            hasher.update(&slot.slot_index.to_le_bytes());
            hasher.update(slot.member.as_bytes());
            hasher.update(&[Self::accessor_tag(slot.accessor)]);
            hasher.update(&[u8::from(slot.sealed)]);
            if let Some(symbol) = &slot.function_symbol {
                hasher.update(symbol.as_bytes());
            }
        }
        let digest = hasher.finalize();
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&digest.as_bytes()[..8]);
        u64::from_le_bytes(buf)
    }

    const fn accessor_tag(kind: Option<PropertyAccessorKind>) -> u8 {
        match kind {
            Some(PropertyAccessorKind::Get) => 1,
            Some(PropertyAccessorKind::Set) => 2,
            Some(PropertyAccessorKind::Init) => 3,
            None => 0,
        }
    }
}

impl ModuleLowering {
    pub(crate) fn finalize_class_vtables(&mut self) -> Vec<ClassVTable> {
        for class in self.class_decls.keys().cloned().collect::<Vec<_>>() {
            let _ = self.ensure_class_vtable_plan(&class);
        }
        let mut names: Vec<_> = self.class_vtable_plans.keys().cloned().collect();
        names.sort();
        let mut tables = Vec::new();
        for name in names {
            let Some(plan) = self.class_vtable_plans.get(&name) else {
                continue;
            };
            if plan.slots.iter().any(|slot| slot.function_symbol.is_none()) {
                continue;
            }
            let slots = plan
                .slots
                .iter()
                .map(|slot| ClassVTableSlot {
                    slot_index: slot.slot_index,
                    member: slot.member.clone(),
                    accessor: slot.accessor,
                    symbol: slot.function_symbol.clone().unwrap_or_default(),
                })
                .collect();
            tables.push(ClassVTable {
                type_name: plan.type_name.clone(),
                symbol: plan.symbol.clone(),
                version: plan.compute_version(),
                slots,
            });
        }
        tables
    }

    pub(crate) fn register_virtual_method(&mut self, meta: LoweredMethodMetadata, symbol: &str) {
        if !dispatch_participates(meta.dispatch) {
            return;
        }
        let plan = self.ensure_class_vtable_plan(&meta.owner);
        let member_key = meta.member.clone();
        if meta.dispatch.is_override {
            if let Some(slot) = plan.slot_mut(&member_key) {
                slot.function_symbol = Some(symbol.to_string());
                if meta.dispatch.is_sealed {
                    slot.sealed = true;
                }
                return;
            }
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "method `{}` in `{}` is marked `override` but no virtual base member was found",
                    member_key, meta.owner
                ),
                span: None,
            });
            return;
        }
        plan.ensure_slot(member_key.clone(), meta.accessor, meta.dispatch.is_sealed);
        if let Some(slot) = plan.slot_mut(&member_key) {
            slot.function_symbol = Some(symbol.to_string());
        }
        self.refresh_class_slot_map(&meta.owner);
    }

    fn refresh_class_slot_map(&mut self, class_name: &str) {
        if let Some(plan) = self.class_vtable_plans.get(class_name) {
            let entry = self
                .class_virtual_slots
                .entry(class_name.to_string())
                .or_default();
            entry.clear();
            for slot in &plan.slots {
                entry.insert(slot.member.clone(), slot.slot_index);
            }
        }
    }

    fn ensure_class_vtable_plan(&mut self, class_name: &str) -> &mut ClassVTablePlan {
        if !self.class_vtable_plans.contains_key(class_name) {
            let plan = self.build_class_vtable_plan(class_name);
            self.class_vtable_plans.insert(class_name.to_string(), plan);
            self.refresh_class_slot_map(class_name);
        }
        self.class_vtable_plans
            .get_mut(class_name)
            .expect("class vtable plan missing after insertion")
    }

    fn build_class_vtable_plan(&mut self, class_name: &str) -> ClassVTablePlan {
        if self.class_vtable_in_progress.contains(class_name) {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "cyclic class inheritance detected while building vtable for `{class_name}`"
                ),
                span: None,
            });
            return ClassVTablePlan::new(class_name);
        }
        self.class_vtable_in_progress.insert(class_name.to_string());
        let mut plan = ClassVTablePlan::new(class_name);
        if let Some(base) = self.primary_class_base(class_name) {
            if base != class_name {
                let base_plan = self.ensure_class_vtable_plan(&base).clone();
                plan.inherit_from(&base_plan);
            }
        }
        if let Some(class_decl) = self.class_decls.get(class_name).cloned() {
            self.populate_plan_from_class(&mut plan, &class_decl);
        }
        self.class_vtable_in_progress.remove(class_name);
        plan
    }

    fn populate_plan_from_class(&mut self, plan: &mut ClassVTablePlan, class: &ClassDecl) {
        for member in &class.members {
            match member {
                ClassMember::Method(method) => {
                    let is_static = method
                        .modifiers
                        .iter()
                        .any(|modifier| modifier.eq_ignore_ascii_case("static"));
                    if is_static || method.dispatch.is_override {
                        continue;
                    }
                    if dispatch_participates(method.dispatch) {
                        plan.ensure_slot(method.name.clone(), None, method.dispatch.is_sealed);
                    }
                }
                ClassMember::Property(property) => {
                    if property.is_static {
                        continue;
                    }
                    for accessor in &property.accessors {
                        if accessor.dispatch.is_override {
                            continue;
                        }
                        if dispatch_participates(accessor.dispatch) {
                            let accessor_name = property.accessor_method_name(accessor.kind);
                            plan.ensure_slot(
                                accessor_name,
                                Some(accessor.kind),
                                accessor.dispatch.is_sealed,
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn primary_class_base(&self, class_name: &str) -> Option<String> {
        let bases = self.class_bases.get(class_name)?;
        bases.iter().find_map(|candidate| {
            self.type_layouts
                .class_layout_info(candidate)
                .map(|_| candidate.clone())
        })
    }
}

#[cfg(test)]
impl ClassVTablePlan {
    pub(super) fn testing_with_slot(type_name: &str, member: &str) -> Self {
        let mut plan = Self::new(type_name);
        plan.ensure_slot(member.to_string(), None, false);
        plan
    }
}
