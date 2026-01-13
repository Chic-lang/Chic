namespace Std.Text.Xml;
import Std.Testing;
public class XmlWriter
{
    public static XmlWriter Create(str output) {
        return new XmlWriter();
    }
}

testcase Given_xml_writer_create_returns_instance_When_executed_Then_xml_writer_create_returns_instance()
{
    let writer = XmlWriter.Create("output");
    Assert.That(writer).IsNotNull();
}
