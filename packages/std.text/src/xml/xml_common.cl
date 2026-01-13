namespace Std.Text.Xml;
import Std.Testing;
public class XmlException : Std.Exception
{
    public init() : super() {
    }
    public init(string message) : super(message) {
    }
}

testcase Given_xml_exception_constructors_When_executed_Then_xml_exception_constructors()
{
    let ex = new XmlException();
    Assert.That(ex).IsNotNull();
    let _ = ex;
}

testcase Given_xml_exception_message_constructor_When_executed_Then_xml_exception_message_constructor()
{
    let ex2 = new XmlException("message");
    Assert.That(ex2).IsNotNull();
    let _ = ex2;
}
