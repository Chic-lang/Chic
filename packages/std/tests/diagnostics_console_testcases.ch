namespace Std;
import Std.Core;
import Std.Diagnostics;
import Std.Platform.IO;
import Std.Testing;
testcase Given_console_in_read_on_empty_stdin_When_executed_Then_returns_eof()
{
    Assert.That(Console.In.Read()).IsEqualTo(- 1);
    Assert.That(Console.In.ReadLine()).IsNull();
    Assert.That(Console.In.IsTerminal).IsFalse();
}
testcase Given_console_out_write_and_newline_When_executed_Then_string_writer_captures()
{
    var writer = new StringWriter();
    Console.SetOut(writer);
    Console.NewLine = "\n";
    Console.Out.Write("a");
    Console.Out.WriteLine("b");
    Assert.That(writer.ToString()).Contains("a");
    Assert.That(writer.ToString()).Contains("b");
}
testcase Given_trace_listener_fail_and_write_line_When_executed_Then_does_not_throw()
{
    var writer = new StringWriter();
    var listener = new DefaultTraceListener(false);
    Console.SetOut(writer);
    listener.Write("x");
    listener.WriteLine("y");
    listener.Fail("fail", null);
    listener.Flush();
    listener.Close();
    Assert.That(writer.ToString()).Contains("x");
}
testcase Given_file_trace_listener_write_flush_close_When_executed_Then_does_not_throw()
{
    let path = "obj/std_file_trace_listener.log";
    var listener = new FileTraceListener(path, false);
    listener.Write("hello");
    listener.WriteLine("world");
    listener.Flush();
    listener.Close();
    listener.Close();
    listener.dispose();
}
testcase Given_trace_listeners_collection_add_remove_When_executed_Then_collection_updates()
{
    let listeners = Trace.Listeners;
    let before = listeners.Count;
    let extra = new ConsoleTraceListener();
    listeners.Add(extra);
    Assert.That(listeners.Count >= before).IsTrue();
    Assert.That(listeners.Remove(extra)).IsTrue();
    Assert.That(listeners.Count).IsEqualTo(before);
}
testcase Given_boolean_switch_override_values_When_executed_Then_enabled_matches_override()
{
    Switches.SetOverride("CHIC_TEST_BOOL_SWITCH_1", "yes");
    let on = new BooleanSwitch("CHIC_TEST_BOOL_SWITCH_1", "desc");
    Assert.That(on.Enabled).IsTrue();
    Switches.SetOverride("CHIC_TEST_BOOL_SWITCH_2", "off");
    let off = new BooleanSwitch("CHIC_TEST_BOOL_SWITCH_2", "desc", true);
    Assert.That(off.Enabled).IsFalse();
    Switches.SetOverride("CHIC_TEST_BOOL_SWITCH_3", "maybe");
    let fallback = new BooleanSwitch("CHIC_TEST_BOOL_SWITCH_3", "desc", true);
    Assert.That(fallback.Enabled).IsTrue();
}
testcase Given_trace_switch_override_values_When_executed_Then_level_matches_override()
{
    Switches.SetOverride("CHIC_TEST_TRACE_SWITCH_1", "warning");
    let warning = new TraceSwitch("CHIC_TEST_TRACE_SWITCH_1", "desc");
    Assert.That(warning.TraceWarning).IsTrue();
    Assert.That(warning.TraceInfo).IsFalse();
    Switches.SetOverride("CHIC_TEST_TRACE_SWITCH_2", "trace");
    let verbose = new TraceSwitch("CHIC_TEST_TRACE_SWITCH_2", "desc");
    Assert.That(verbose.TraceVerbose).IsTrue();
}
testcase Given_trace_listener_fail_null_message_and_detail_When_executed_Then_does_not_throw()
{
    var listener = new TraceListener();
    listener.Fail(null, null);
    listener.Fail(null, "detail");
    listener.Fail("message", "detail");
}
testcase Given_assert_failed_exception_detail_message_When_executed_Then_preserved()
{
    let ex = new AssertFailedException("boom", "detail");
    Assert.That(ex.DetailMessage).IsEqualTo("detail");
    let _ = ex;
}
