use std::fs;

use chic::doc::extensions::{DocContext, DocTagHandler, HandlerOutput, LinkResolver, ResolvedLink};
use chic::doc::model::{BlockContent, LinkNode, LinkTarget};
use chic::doc::{
    DocGenerationOptions, DocOutputLayout, DocTemplate, generate_markdown, register_link_resolver,
    register_tag_handler,
};
use chic::frontend::metadata::collect_reflection_tables;
use chic::frontend::parser::parse_module;
use expect_test::expect;
use tempfile::tempdir;

struct TestCalloutHandler;

impl DocTagHandler for TestCalloutHandler {
    fn handle(&self, element: &roxmltree::Node, _ctx: &DocContext) -> Option<HandlerOutput> {
        let text = element
            .text()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("note");
        Some(HandlerOutput::block(BlockContent::BlockQuote(vec![
            BlockContent::Paragraph(vec![chic::doc::model::InlineContent::Text(
                text.to_string(),
            )]),
        ])))
    }

    fn supports(&self, name: &str) -> bool {
        name.eq_ignore_ascii_case("acme:note")
    }
}

struct TestLinkResolver;

impl LinkResolver for TestLinkResolver {
    fn resolve(&self, link: &LinkNode, _ctx: &DocContext) -> Option<ResolvedLink> {
        let target = match &link.target {
            LinkTarget::Cref(cref) | LinkTarget::Plain(cref) => {
                format!("https://docs.example.test/{cref}")
            }
            LinkTarget::Url(url) => url.clone(),
        };
        Some(ResolvedLink {
            text: link.text.clone().unwrap_or_else(|| target.clone()),
            target: Some(target),
        })
    }
}

#[test]
fn generates_markdown_from_xml_docs() {
    let source = r#"
namespace Samples;

/// <summary>Widget container.</summary>
/// <remarks><para>Core widget type.</para><list type="bullet"><item><term>First</term><description>entry</description></item></list></remarks>
public class Widget<T>
{
    /// <summary>Stored value.</summary>
    /// <typeparam name="T">payload type</typeparam>
    public T Value;

    /// <summary>Adds <paramref name="delta"/> to the current value.</summary>
    /// <param name="delta">Amount to add.</param>
    /// <returns>New value.</returns>
    /// <example><code lang="chic">let widget = Widget{ Value: delta };</code></example>
    /// <seealso cref="Samples::Widget{T}"/>
    /// <chic:note>Be careful with overflow.</chic:note>
    public T Add(T delta)
    {
        return delta;
    }
}
"#;

    let parse = parse_module(source).unwrap_or_else(|err| panic!("{:?}", err.diagnostics()));
    let tables = collect_reflection_tables(&parse.module);
    let dir = tempdir().expect("tempdir");
    let mut options = DocGenerationOptions::default();
    options.output_root = dir.path().to_path_buf();
    options.banner = None;
    options.front_matter_template = None;
    options.template = DocTemplate::none();
    options.layout = DocOutputLayout::PerType;
    let result = generate_markdown(&tables, &options).expect("generate docs");
    assert!(
        result.diagnostics.is_empty(),
        "unexpected doc diagnostics: {:?}",
        result.diagnostics
    );
    let generated = fs::read_to_string(dir.path().join("Samples").join("Widget.md"))
        .expect("generated widget docs");
    let expected = expect![[r#"
        # Class Widget

        `Widget<T>`

        Widget container.


        ## Remarks

        - First: entry



        ## Members

        ### Field Value

        `Value: T`

        Stored value.


        ## Type Parameters

        | Name | Description |
        | --- | --- |
        | `T` | payload type |

        ### Method Add

        `Add(delta: T) -> T`

        Adds `delta` to the current value.


        ## Parameters

        | Name | Description |
        | --- | --- |
        | `delta` | Amount to add. |

        ## Returns

        New value.


        ## Examples

        ```chic
        let widget = Widget{ Value: delta };
        ```

        ## Remarks

        > Note: Be careful with overflow.
        > 


        ## See Also

        - [Samples::Widget{T}](#samples--widget-t)"#]];
    expected.assert_eq(generated.trim());
}

#[test]
fn applies_custom_extensions_and_front_matter() {
    register_tag_handler("acme:note", std::sync::Arc::new(TestCalloutHandler));
    register_link_resolver("acme", std::sync::Arc::new(TestLinkResolver));

    let source = r#"
namespace Samples;

/// <summary>Extension target.</summary>
public class Gizmo
{
    /// <summary>Runs the gizmo.</summary>
    /// <see cref="Samples::Gizmo.Run"/>
    /// <acme:note>custom callout</acme:note>
    public void Run() { }
}
"#;
    let parse = parse_module(source).unwrap_or_else(|err| panic!("{:?}", err.diagnostics()));
    let tables = collect_reflection_tables(&parse.module);
    let dir = tempdir().expect("tempdir");
    let mut options = DocGenerationOptions::default();
    options.output_root = dir.path().to_path_buf();
    options.banner = None;
    options.layout = DocOutputLayout::SingleFile;
    options.front_matter_template = Some("title: {{title}}\nkind: {{kind}}\n".to_string());
    options.template = DocTemplate::none();
    options.tag_handlers = vec!["acme:note".to_string()];
    options.link_resolver = Some("acme".to_string());

    let result = generate_markdown(&tables, &options).expect("generate docs");
    assert!(
        result.diagnostics.is_empty(),
        "unexpected doc diagnostics: {:?}",
        result.diagnostics
    );
    let generated = fs::read_to_string(dir.path().join("API.md")).expect("generated docs");
    let expected = expect![[r#"
        title: Gizmo
        kind: Class

        # Class Gizmo

        `Gizmo`

        Extension target.



        ## Members

        ### Method Run

        `Run() -> void`

        Runs the gizmo.


        ## Remarks

        > custom callout
        > 


        ## See Also

        - [https://docs.example.test/Samples::Gizmo.Run](https://docs.example.test/Samples::Gizmo.Run)"#]];
    expected.assert_eq(generated.trim());
}
