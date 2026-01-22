namespace Std;
import Std.Core;
import Std.Testing;
import Std.Collections;
import Std.Hashing;
import Std.Sync;
private static bool IsEven(in int value) {
    return value % 2 == 0;
}
testcase Given_string_reader_reads_lines_When_executed_Then_expected_lines_returned()
{
    var reader = new StringReader("a\nb\r\nc");
    Assert.That(reader.ReadLine()).IsEqualTo("a");
    Assert.That(reader.ReadLine()).IsEqualTo("b");
    Assert.That(reader.ReadLine()).IsEqualTo("c");
    Assert.That(reader.ReadLine()).IsNull();
}
testcase Given_console_set_out_and_write_line_When_executed_Then_string_writer_receives_text()
{
    var writer = new StringWriter();
    Console.SetOut(writer);
    let originalNewLine = Console.NewLine;
    Console.NewLine = "\n";
    Console.WriteLine("hello");
    Console.NewLine = originalNewLine;
    Assert.That(writer.ToString()).Contains("hello");
}
testcase Given_terminal_capabilities_detect_When_executed_Then_defaults_are_stable()
{
    let caps = TerminalCapabilities.Detect();
    Assert.That(caps.SupportsReadKey).IsFalse();
    Assert.That(caps.SupportsSizing).IsFalse();
}
testcase Given_hashmap_basic_operations_When_executed_Then_expected_results_observed()
{
    var map = HashMap <int, string >.WithCapacity(4usize);
    var status = map.Insert(1, "one", out var previous);
    let key1 = 1;
    let key2 = 2;
    Assert.That(status == HashMapError.Success).IsTrue();
    Assert.That(previous.IsNone()).IsTrue();
    Assert.That(map.ContainsKey(in key1)).IsTrue();
    Assert.That(map.ContainsKey(in key2)).IsFalse();
    Assert.That(map.Get(in key1).IsSome(out var value)).IsTrue();
    Assert.That(value).IsEqualTo("one");
    Assert.That(map.Remove(in key1)).IsTrue();
    Assert.That(map.ContainsKey(in key1)).IsFalse();
    map.dispose();
}
testcase Given_hashset_drain_and_filter_When_executed_Then_values_are_drained()
{
    var hashSet = new HashSet <int >();
    var inserted = false;
    let _ = hashSet.Insert(1, out inserted);
    let _ = hashSet.Insert(2, out inserted);
    let _ = hashSet.Insert(3, out inserted);
    var filter = hashSet.DrainFilter(IsEven);
    Assert.That(filter.Next(out var drained)).IsTrue();
    Assert.That(drained).IsEqualTo(2);
    filter.dispose();
    var drain = hashSet.Drain();
    Assert.That(drain.Next(out var any)).IsTrue();
    drain.dispose();
    hashSet.dispose();
}
testcase Given_sync_lock_and_mutex_When_executed_Then_guards_release()
{
    var lockObj = new Lock();
    var lockGuard = lockObj.Enter();
    Assert.That(lockGuard.Held).IsTrue();
    lockGuard.Release();
    lockObj.dispose();
    var mutex = new Mutex <int >(123);
    var guard = mutex.Lock();
    Assert.That(guard.Held).IsTrue();
    Assert.That(guard.Value).IsEqualTo(123);
    guard.Release();
    mutex.dispose();
}
testcase Given_arc_roundtrip_raw_When_executed_Then_borrow_reads_value()
{
    var arc = new Arc <int >(42);
    Assert.That(arc.Borrow()).IsEqualTo(42);
    let raw = arc.IntoRaw();
    var restored = Arc <int >.FromRaw(raw);
    arc = CoreIntrinsics.DefaultValue <Arc <int >> ();
    Assert.That(restored.Borrow()).IsEqualTo(42);
    restored.dispose();
}
testcase Given_uri_to_string_and_escape_roundtrip_When_executed_Then_components_preserved()
{
    let uri = new Uri("http://example.com:8080/path?x=1#frag");
    Assert.That(uri.IsAbsoluteUri).IsTrue();
    Assert.That(uri.ToString()).Contains("example.com");
    Assert.That(uri.GetHashCode() == uri.GetHashCode()).IsTrue();
    let escaped = UriEscape.EscapeComponent("a b", UriEscapeComponent.Path, false);
    Assert.That(escaped).IsEqualTo("a%20b");
    let unescaped = UriEscape.UnescapeString(escaped, false);
    Assert.That(unescaped).IsEqualTo("a b");
}
testcase Given_uuid_equality_and_hashcode_When_executed_Then_behaves_as_value_type()
{
    let left = new Uuid(1u, 2u16, 3u16, 4u8, 5u8, 6u8, 7u8, 8u8, 9u8, 10u8, 11u8);
    let _ = left.Clone();
    let right = left;
    Assert.That(left == right).IsTrue();
    Assert.That(left.Equals(right)).IsTrue();
    Assert.That(left.GetHashCode() == right.GetHashCode()).IsTrue();
}
testcase Given_string_equals_and_hash_When_executed_Then_consistent()
{
    Assert.That("abc" == "abc").IsTrue();
    Assert.That("abc".Equals("abc")).IsTrue();
    Assert.That("abc".GetHashCode() == "abc".GetHashCode()).IsTrue();
}
testcase Given_hashing_write_u64_When_executed_Then_hasher_advances()
{
    var hasher = new DefaultHasher();
    let initial = hasher.Finish();
    Hashing.WriteU64 <DefaultHasher >(ref hasher, 123ul);
    Assert.That(hasher.Finish() != initial).IsTrue();
}
