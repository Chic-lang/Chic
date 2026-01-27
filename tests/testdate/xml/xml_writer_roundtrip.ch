namespace Exec;

import Std.Text.Xml;
import Std.IO;

public static class Program
{
    public static int Main()
    {
        var ms = new MemoryStream();
        var settings = new XmlWriterSettings();
        settings.Indent = true;
        var writer = XmlWriter.Create(ms, settings);
        writer.WriteStartElement("", "root", "");
        writer.WriteAttributeString("", "id", "", "123");
        writer.WriteStartElement("p", "item", "urn:test");
        writer.WriteString("hello");
        writer.WriteEndElement();
        writer.WriteEndElement();
        writer.WriteEndDocument();
        ms.ResetPosition();
        var reader = XmlReader.Create(ms, new XmlReaderSettings());
        var names = Std.Runtime.StringRuntime.Create();
        while (reader.Read())
        {
            if (reader.NodeType == XmlNodeType.Element)
            {
                names += reader.Name + ";";
            }
        }
        Std.Console.WriteLine(names);
        return 0;
    }
}
