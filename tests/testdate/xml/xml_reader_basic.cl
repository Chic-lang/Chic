namespace Exec;

import Std.Text.Xml;
import Std.IO;

public static class Program
{
    public static int Main()
    {
        let xml = "<root><child attr=\"v\">text</child><!--c--></root>";
        var reader = XmlReader.Create(xml, new XmlReaderSettings());
        var tokens = Std.Runtime.StringRuntime.Create();
        while (reader.Read())
        {
            tokens += reader.NodeType.ToString();
            if (reader.NodeType == XmlNodeType.Element)
            {
                tokens += ":" + reader.Name;
            }
            if (reader.NodeType == XmlNodeType.Text)
            {
                tokens += ":" + reader.Value;
            }
            tokens += ";";
        }
        Std.Console.WriteLine(tokens);
        return 0;
    }
}
