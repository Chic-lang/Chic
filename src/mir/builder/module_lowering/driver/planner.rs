use super::super::super::Module;

#[derive(Debug, Clone)]
pub(crate) struct LowerPlan {
    pub(crate) item_units: Option<Vec<usize>>,
}

pub(crate) struct LowerPlanner;

impl LowerPlanner {
    pub fn new() -> Self {
        Self
    }

    pub fn plan(&self, module: &Module, item_units: Option<&[usize]>) -> LowerPlan {
        let planned_units = item_units.map(|units| units.to_vec());
        // If provided, ensure the unit count aligns with the item count to avoid surprises.
        if let Some(ref units) = planned_units {
            debug_assert_eq!(
                units.len(),
                module.items.len(),
                "item_units length should match module items"
            );
        }
        LowerPlan {
            item_units: planned_units,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::parser::parse_module;

    #[test]
    fn plans_without_units_returns_none() {
        let parsed = parse_module("public void Main() { }").expect("parse module");
        let plan = LowerPlanner::new().plan(&parsed.module, None);
        assert!(plan.item_units.is_none());
    }

    #[test]
    fn plans_provided_units() {
        let parsed = parse_module(
            r#"
public void A() {}
public void B() {}
"#,
        )
        .expect("parse module");
        let plan = LowerPlanner::new().plan(&parsed.module, Some(&[0, 1]));
        assert_eq!(plan.item_units.as_deref(), Some(&[0, 1][..]));
    }
}
