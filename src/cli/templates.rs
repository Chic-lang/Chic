/// Supported `chic init` templates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TemplateKind {
    App,
}

pub struct TemplateAsset {
    pub path: &'static str,
    pub contents: &'static str,
}

pub struct RenderedAsset {
    pub relative_path: &'static str,
    pub contents: String,
}

impl TemplateKind {
    pub fn canonical_name(self) -> &'static str {
        match self {
            TemplateKind::App => "app",
        }
    }

    pub fn aliases(self) -> &'static [&'static str] {
        match self {
            TemplateKind::App => &["app", "app-console", "console"],
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            TemplateKind::App => "App (console)",
        }
    }

    pub fn default_project_name(self) -> &'static str {
        match self {
            TemplateKind::App => "MyApp",
        }
    }

    pub fn assets(self) -> &'static [TemplateAsset] {
        match self {
            TemplateKind::App => APP_TEMPLATE,
        }
    }
}

pub fn resolve_template(name: &str) -> Option<TemplateKind> {
    let requested = name.trim();
    if requested.is_empty() {
        return None;
    }
    let lower = requested.to_ascii_lowercase();
    for kind in [TemplateKind::App] {
        if kind
            .aliases()
            .iter()
            .any(|alias| alias.eq_ignore_ascii_case(&lower))
        {
            return Some(kind);
        }
    }
    None
}

pub fn available_templates() -> Vec<&'static str> {
    [TemplateKind::App]
        .iter()
        .map(|kind| kind.canonical_name())
        .collect()
}

pub fn render(kind: TemplateKind, project_name: &str) -> Vec<RenderedAsset> {
    let namespace = derive_namespace(project_name);
    kind.assets()
        .iter()
        .map(|asset| RenderedAsset {
            relative_path: asset.path,
            contents: render_content(asset.contents, project_name, &namespace),
        })
        .collect()
}

fn render_content(template: &str, project_name: &str, namespace: &str) -> String {
    template
        .replace("{{project_name}}", project_name)
        .replace("{{project_namespace}}", namespace)
}

fn derive_namespace(project_name: &str) -> String {
    let mut parts = Vec::new();
    let mut current = String::new();
    for ch in project_name.chars() {
        if ch.is_ascii_alphanumeric() {
            current.push(ch);
        } else if !current.is_empty() {
            parts.push(current);
            current = String::new();
        }
    }
    if !current.is_empty() {
        parts.push(current);
    }

    if parts.is_empty() {
        return TemplateKind::App.default_project_name().to_string();
    }

    let mut name = String::new();
    for part in parts {
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            name.push(first.to_ascii_uppercase());
            for ch in chars {
                name.push(ch);
            }
        }
    }

    if name.chars().next().map_or(false, |ch| ch.is_ascii_digit()) {
        name.insert(0, '_');
    }

    if name.is_empty() {
        TemplateKind::App.default_project_name().to_string()
    } else {
        name
    }
}

static APP_TEMPLATE: &[TemplateAsset] = &[
    TemplateAsset {
        path: "manifest.yaml",
        contents: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/app/manifest.yaml"
        )),
    },
    TemplateAsset {
        path: "src/App.cl",
        contents: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/app/src/App.cl"
        )),
    },
    TemplateAsset {
        path: "tests/AppTests.cl",
        contents: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/app/tests/AppTests.cl"
        )),
    },
    TemplateAsset {
        path: "README.md",
        contents: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/app/README.md"
        )),
    },
    TemplateAsset {
        path: "docs/README.md",
        contents: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/app/docs/README.md"
        )),
    },
    TemplateAsset {
        path: ".github/workflows/ci.yml",
        contents: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/app/.github/workflows/ci.yml"
        )),
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_template_aliases_case_insensitively() {
        for alias in ["app", "APP", "App", "app-console", "console"] {
            assert_eq!(
                resolve_template(alias),
                Some(TemplateKind::App),
                "alias `{alias}` should resolve"
            );
        }
        assert!(resolve_template("").is_none());
        assert!(resolve_template("unknown").is_none());
    }

    #[test]
    fn derives_namespace_from_project_name() {
        assert_eq!(derive_namespace("demo-app"), "DemoApp");
        assert_eq!(derive_namespace("123app"), "_123app");
        assert_eq!(
            derive_namespace(""),
            TemplateKind::App.default_project_name()
        );
    }

    #[test]
    fn renders_placeholders() {
        let rendered = render(TemplateKind::App, "SampleApp");
        let manifest = rendered
            .iter()
            .find(|asset| asset.relative_path == "manifest.yaml")
            .expect("manifest asset");
        assert!(
            manifest.contents.contains("SampleApp"),
            "project name placeholder should render"
        );
        let app = rendered
            .iter()
            .find(|asset| asset.relative_path.ends_with("App.cl"))
            .expect("App asset");
        assert!(
            app.contents.contains("namespace SampleApp;"),
            "namespace placeholder should render"
        );
    }
}
