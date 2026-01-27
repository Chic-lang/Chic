namespace Std.Text.Xml;
import Std.Testing;
public class XmlReader
{
    public static XmlReader Create(string input) {
        return new XmlReader();
    }
}
testcase Given_xml_reader_create_returns_instance_When_executed_Then_xml_reader_create_returns_instance()
{
    let reader = XmlReader.Create("<root />");
    Assert.That(reader).IsNotNull();
}
