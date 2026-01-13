# Std.Text.Xml overview

Std.Text.Xml provides streaming XML parsing and writing for Chic. The stack is UTF-8–first, deterministic across backends, and secure-by-default (DTD disabled).

## Highlights

- `XmlReader` streams tokens without building a DOM. It exposes node type, names, attributes, depth, and values, plus helpers like `ReadElementContentAsString`.
- `XmlWriter` streams UTF-8 XML with proper escaping, optional indentation, and namespace declarations.
- Settings classes let you tune whitespace/comment handling, maximum depth, indentation, and whether inputs/outputs are closed.
- DTD and external entity processing are disabled by default and raise `XmlException` if encountered.

## Quick examples

Parse from a string:

```chic
var reader = Std.Text.Xml.XmlReader.Create("<root><child attr=\"v\">text</child></root>");
while (reader.Read())
{
    if (reader.NodeType == Std.Text.Xml.XmlNodeType.Element)
    {
        Std.Console.WriteLine(reader.Name);
    }
}
```

Write a document:

```chic
var ms = new Std.IO.MemoryStream();
var settings = new Std.Text.Xml.XmlWriterSettings();
settings.Indent = true;
var writer = Std.Text.Xml.XmlWriter.Create(ms, settings);
writer.WriteStartElement("", "root", "");
writer.WriteAttributeString("", "id", "", "123");
writer.WriteString("hello");
writer.WriteEndElement();
writer.WriteEndDocument();
```
