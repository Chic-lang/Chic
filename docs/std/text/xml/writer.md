# XmlWriter

`Std.Text.Xml.XmlWriter` streams UTF-8 XML with proper escaping, optional indentation, and namespace declarations.

## Settings

- `EncodingName` (default `utf-8`)
- `Indent` / `IndentChars` / `NewLineChars`
- `OmitXmlDeclaration`
- `CloseOutput` (reserved)

## Core API

- `WriteStartElement(prefix, localName, nsUri)`
- `WriteAttributeString(prefix, localName, nsUri, value)`
- `WriteString`, `WriteCData`, `WriteComment`, `WriteProcessingInstruction`, `WriteRaw`
- `WriteEndElement` / `WriteFullEndElement`
- `WriteStartDocument`, `WriteEndDocument`, `Flush`/`FlushAsync`

## Example

```chic
var ms = new Std.IO.MemoryStream();
var settings = new Std.Text.Xml.XmlWriterSettings();
settings.Indent = true;
var writer = Std.Text.Xml.XmlWriter.Create(ms, settings);
writer.WriteStartElement("", "config", "");
writer.WriteAttributeString("", "version", "", "1");
writer.WriteStartElement("ns", "entry", "urn:cfg");
writer.WriteString("value");
writer.WriteEndElement();
writer.WriteEndElement();
writer.WriteEndDocument();
```

The writer escapes `&`, `<`, `>`, `'`, and `"` automatically. Namespace declarations are emitted when a prefix/URI pair is first seen in scope.
