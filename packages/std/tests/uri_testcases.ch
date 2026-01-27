namespace Std;
import Std.Core;
import Std.Testing;
testcase Given_uri_parses_userinfo_host_port_When_executed_Then_components_match()
{
    let uri = new Uri("http://user:pass@example.com:8080/path?x=1#frag");
    Assert.That(uri.IsAbsoluteUri).IsTrue();
    Assert.That(uri.Scheme).IsEqualTo("http");
    Assert.That(uri.UserInfo).IsEqualTo("user:pass");
    Assert.That(uri.Host).IsEqualTo("example.com");
    Assert.That(uri.Port).IsEqualTo(8080);
    Assert.That(uri.AbsolutePath).IsEqualTo("/path");
    Assert.That(uri.Query).IsEqualTo("?x=1");
    Assert.That(uri.Fragment).IsEqualTo("#frag");
    Assert.That(uri.Authority).Contains("example.com");
}
testcase Given_uri_parses_ipv6_host_and_port_When_executed_Then_host_type_is_ipv6()
{
    let uri = new Uri("http://[::1]:443/");
    Assert.That(uri.HostNameType == UriHostNameType.IPv6).IsTrue();
    Assert.That(uri.IsDefaultPort).IsTrue();
    Assert.That(uri.Port).IsEqualTo(443);
    Assert.That(uri.Host).IsEqualTo("::1");
}
testcase Given_uri_parses_ipv4_loopback_When_executed_Then_is_loopback_true()
{
    let uri = new Uri("http://127.0.0.1/");
    Assert.That(uri.HostNameType == UriHostNameType.IPv4).IsTrue();
    Assert.That(uri.IsLoopback).IsTrue();
    Assert.That(uri.Host).IsEqualTo("127.0.0.1");
}
testcase Given_uri_removes_dot_segments_When_resolving_relative_Then_path_is_normalized()
{
    let baseUri = new Uri("http://example.com/a/b/c");
    let resolved = new Uri(baseUri, "../d/./e");
    Assert.That(resolved.AbsolutePath).IsEqualTo("/a/d/e");
    Assert.That(resolved.ToString()).Contains("/a/d/e");
}
testcase Given_uri_try_create_invalid_ipv6_When_executed_Then_returns_false()
{
    let ok = Uri.TryCreate("http://[::1", UriKind.RelativeOrAbsolute, out var result);
    Assert.That(ok).IsFalse();
    Assert.That(result).IsNull();
}
testcase Given_uri_try_create_invalid_port_When_executed_Then_returns_false()
{
    let ok = Uri.TryCreate("http://example.com:999999/path", UriKind.RelativeOrAbsolute, out var result);
    Assert.That(ok).IsFalse();
    Assert.That(result).IsNull();
}
testcase Given_uri_file_unc_When_executed_Then_is_unc_and_local_path_has_host()
{
    let uri = new Uri("file://server/share/file.txt");
    Assert.That(uri.IsFile).IsTrue();
    Assert.That(uri.IsUnc).IsTrue();
    Assert.That(uri.Host).IsEqualTo("server");
    Assert.That(uri.LocalPath).Contains("//server/");
}
testcase Given_uri_file_no_authority_When_executed_Then_is_file_and_path_is_preserved()
{
    let uri = new Uri("file:///tmp/example.txt");
    Assert.That(uri.IsFile).IsTrue();
    Assert.That(uri.IsUnc).IsFalse();
    Assert.That(uri.AbsolutePath).IsEqualTo("/tmp/example.txt");
    Assert.That(uri.LocalPath).IsEqualTo("/tmp/example.txt");
}
testcase Given_uri_equals_is_case_insensitive_on_scheme_and_host_When_executed_Then_equal_true()
{
    let left = new Uri("HTTP://EXAMPLE.COM/path");
    let right = new Uri("http://example.com/path");
    Assert.That(left.Equals(right)).IsTrue();
    Assert.That(left == right).IsTrue();
    Assert.That(left.GetHashCode() == right.GetHashCode()).IsTrue();
}
testcase Given_uri_escape_unescape_safe_flag_When_executed_Then_reserved_kept_when_safe()
{
    let encoded = "%2F";
    let safe = UriEscape.UnescapeString(encoded, true);
    Assert.That(safe).IsEqualTo("%2F");
    let unsafeValue = UriEscape.UnescapeString(encoded, false);
    Assert.That(unsafeValue).IsEqualTo("/");
}
testcase Given_uri_escape_unescape_roundtrip_When_executed_Then_space_escapes_and_unescapes()
{
    let escaped = UriEscape.EscapeComponent("a b", UriEscapeComponent.Query, false);
    Assert.That(escaped).IsEqualTo("a%20b");
    let unescaped = UriEscape.UnescapeString(escaped, false);
    Assert.That(unescaped).IsEqualTo("a b");
}
testcase Given_uri_escape_preserves_existing_escapes_When_preserve_true_Then_keeps_literal()
{
    let escaped = UriEscape.EscapeComponent("%2F", UriEscapeComponent.Path, true);
    Assert.That(escaped).IsEqualTo("%2F");
}
testcase Given_uri_unescape_ignores_invalid_percent_sequences_When_executed_Then_string_preserved()
{
    let value = "a%ZZb%2";
    let unescaped = UriEscape.UnescapeString(value, false);
    Assert.That(unescaped).IsEqualTo(value);
}
testcase Given_uri_hex_escape_out_of_range_When_executed_Then_throws_format_exception()
{
    Assert.Throws <UriFormatException >(() => {
        let _ = UriEscape.HexEscape('☃');
    }
    );
}
testcase Given_uri_segments_and_path_and_query_When_executed_Then_segments_match()
{
    let uri = new Uri("http://example.com/a/b/c?x=1");
    let segments = uri.Segments;
    Assert.That(segments.Length).IsEqualTo(4);
    Assert.That(segments[0]).IsEqualTo("/");
    Assert.That(segments[1]).IsEqualTo("a/");
    Assert.That(segments[2]).IsEqualTo("b/");
    Assert.That(segments[3]).IsEqualTo("c");
    Assert.That(uri.PathAndQuery).IsEqualTo("/a/b/c?x=1");
}
testcase Given_uri_get_left_part_variants_When_executed_Then_expected_prefixes_returned()
{
    let uri = new Uri("http://example.com:8080/a/b?x=1#f");
    Assert.That(uri.GetLeftPart(UriPartial.Scheme)).IsEqualTo("http:");
    Assert.That(uri.GetLeftPart(UriPartial.Authority)).IsEqualTo("http://example.com:8080");
    Assert.That(uri.GetLeftPart(UriPartial.Path)).IsEqualTo("http://example.com:8080/a/b");
    Assert.That(uri.GetLeftPart(UriPartial.Query)).IsEqualTo("http://example.com:8080/a/b?x=1");
}
testcase Given_uri_get_components_for_common_components_When_executed_Then_returns_expected_values()
{
    let uri = new Uri("http://example.com:8080/a/b?x=1#f");
    Assert.That(uri.GetComponents(UriComponents.Scheme, UriFormat.Unescaped)).IsEqualTo("http");
    Assert.That(uri.GetComponents(UriComponents.Authority, UriFormat.Unescaped)).IsEqualTo("example.com:8080");
    Assert.That(uri.GetComponents(UriComponents.PathAndQuery, UriFormat.Unescaped)).IsEqualTo("/a/b?x=1");
    Assert.That(uri.GetComponents(UriComponents.AbsoluteUri, UriFormat.UriEscaped)).Contains("http://");
}
testcase Given_uri_make_relative_uri_When_executed_Then_relative_contains_query_and_fragment()
{
    let baseUri = new Uri("http://example.com/a/b/");
    let target = new Uri("http://example.com/a/b/c/d?x=1#f");
    let relative = baseUri.MakeRelativeUri(target);
    Assert.That(relative.IsAbsoluteUri).IsFalse();
    Assert.That(relative.OriginalString).IsEqualTo("c/d?x=1#f");
}
testcase Given_uri_escape_unescape_data_string_When_executed_Then_roundtrips_space()
{
    let escaped = Uri.EscapeDataString("a b");
    Assert.That(escaped).IsEqualTo("a%20b");
    let unescaped = Uri.UnescapeDataString(escaped);
    Assert.That(unescaped).IsEqualTo("a b");
}
testcase Given_uri_local_path_on_non_file_When_executed_Then_throws_invalid_operation_exception()
{
    let uri = new Uri("http://example.com/path");
    Assert.Throws <InvalidOperationException >(() => {
        let _ = uri.LocalPath;
    }
    );
}
testcase Given_uri_is_base_of_When_executed_Then_matches_expected()
{
    let baseUri = new Uri("http://example.com/a/b/");
    let target = new Uri("http://example.com/a/b/c");
    Assert.That(baseUri.IsBaseOf(target)).IsTrue();
    let otherHost = new Uri("http://other.example.com/a/b/c");
    Assert.That(baseUri.IsBaseOf(otherHost)).IsFalse();
}
testcase Given_uri_try_create_base_and_relative_variants_When_executed_Then_result_is_resolved()
{
    let baseUri = new Uri("http://example.com/a/b/");
    let ok = Uri.TryCreate(baseUri, "c?x=1#f", out var result);
    Assert.That(ok).IsTrue();
    Assert.That(result).IsNotNull();
    Assert.That(result.AbsolutePath).IsEqualTo("/a/b/c");
    Assert.That(result.Query).IsEqualTo("?x=1");
    Assert.That(result.Fragment).IsEqualTo("#f");
}
testcase Given_uri_mailto_scheme_When_executed_Then_is_absolute_without_authority()
{
    let uri = new Uri("mailto:user@example.com");
    Assert.That(uri.IsAbsoluteUri).IsTrue();
    Assert.That(uri.Scheme).IsEqualTo("mailto");
    Assert.That(uri.Authority).IsEqualTo("");
    Assert.That(uri.AbsolutePath).Contains("user@example.com");
}
