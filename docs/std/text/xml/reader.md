# XmlReader

`Std.Text.Xml.XmlReader` is a forward-only, streaming reader over UTF-8 XML. It does not build a DOM and is suitable for configuration and protocol parsing.

## Settings

- `IgnoreComments` / `IgnoreWhitespace`
- `DtdProcessing` (default false; DTDs raise `XmlException` when disabled)
- `MaxDepth` (default 64)
- `CheckCharacters` (reserved for stricter name/char validation)
- `CloseInput` (not used by the current implementation)

## Core API

- `bool Read()` advances to the next node.
- Node info: `NodeType`, `Name`, `LocalName`, `NamespaceUri`, `Prefix`, `Value`, `Depth`, `EOF`.
- Attributes: `AttributeCount`, `MoveToAttribute(...)`, `MoveToFirstAttribute()`, `MoveToNextAttribute()`, `MoveToElement()`, `GetAttribute(...)`.
- Helpers: `IsStartElement(...)`, `ReadStartElement(...)`, `ReadEndElement()`, `ReadElementContentAsString()`.

## Example

```chic
var reader = Std.Text.Xml.XmlReader.Create("<root><item x=\"1\">text</item></root>");
while (reader.Read())
{
    switch (reader.NodeType)
    {
        case Std.Text.Xml.XmlNodeType.Element:
            Std.Console.WriteLine("start:" + reader.Name);
            break;
        case Std.Text.Xml.XmlNodeType.EndElement:
            Std.Console.WriteLine("end:" + reader.Name);
            break;
        case Std.Text.Xml.XmlNodeType.Text:
            Std.Console.WriteLine("text:" + reader.Value);
            break;
    }
}
```

## Namespaces and security

Namespace declarations (`xmlns`/`xmlns:prefix`) are tracked per-scope; `NamespaceUri` reflects the resolved URI for element and attribute prefixes. DTDs and external entities are rejected unless `DtdProcessing` is explicitly enabled.***
