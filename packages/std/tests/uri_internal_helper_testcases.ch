namespace Std;
import Std.Collections;
import Std.Core;
import Std.Numeric;
import Std.Span;
import Std.Testing;
import Foundation.Collections;
import FVec = Foundation.Collections.Vec;
import FVecIntrinsics = Foundation.Collections.VecIntrinsics;
testcase Given_uri_parse_authority_with_userinfo_host_and_port_When_executed_Then_parts_are_populated()
{
    var parts = CoreIntrinsics.DefaultValue <UriParts >();
    let span = "user:pass@EXAMPLE.com:8080".AsUtf8Span();
    let ok = Uri.ParseAuthority(span, false, ref parts, out var error);
    Assert.That(ok).IsTrue();
    Assert.That(error.Length).IsEqualTo(0);
    Assert.That(parts.UserInfo).IsEqualTo("user:pass");
    Assert.That(parts.Host).IsEqualTo("example.com");
    Assert.That(parts.PortSpecified).IsTrue();
    Assert.That(parts.Port).IsEqualTo(8080);
}
testcase Given_uri_validate_host_with_ipv4_When_executed_Then_host_type_is_ipv4()
{
    let ok = Uri.ValidateHost("127.0.0.1".AsUtf8Span(), out var hostType, out var host, out var idnHost);
    Assert.That(ok).IsTrue();
    Assert.That(hostType == UriHostNameType.IPv4).IsTrue();
    Assert.That(host).IsEqualTo("127.0.0.1");
    Assert.That(idnHost).IsEqualTo(host);
}
testcase Given_uri_is_valid_ipv6_with_loopback_When_executed_Then_returns_true()
{
    Assert.That(Uri.IsValidIPv6("::1".AsUtf8Span())).IsTrue();
}
testcase Given_uri_merge_paths_with_base_and_relative_When_executed_Then_combines_prefix()
{
    let merged = Uri.MergePaths("/a/b/c", "d", true);
    Assert.That(merged).IsEqualTo("/a/b/d");
}
testcase Given_uri_escape_hex_helpers_When_executed_Then_expected_values_returned()
{
    Assert.That(UriEscape.IsHexDigit(NumericUnchecked.ToByte('f'))).IsTrue();
    Assert.That(UriEscape.IsHexDigit(NumericUnchecked.ToByte('g'))).IsFalse();
    Assert.That(UriEscape.FromHex(NumericUnchecked.ToByte('A'))).IsEqualTo(10);
    Assert.That(UriEscape.FromHex(NumericUnchecked.ToByte('0'))).IsEqualTo(0);
}
testcase Given_uri_escape_allowed_sets_When_executed_Then_expected_membership()
{
    Assert.That(UriEscape.IsUnreserved(NumericUnchecked.ToByte('~'))).IsTrue();
    Assert.That(UriEscape.IsSubDelim(NumericUnchecked.ToByte('!'))).IsTrue();
    Assert.That(UriEscape.IsAllowedInPath(NumericUnchecked.ToByte('/'))).IsTrue();
    Assert.That(UriEscape.IsAllowedInQueryOrFragment(NumericUnchecked.ToByte('?'))).IsTrue();
    Assert.That(UriEscape.IsAllowedInUserInfo(NumericUnchecked.ToByte(':'))).IsTrue();
    Assert.That(UriEscape.IsAllowedComponent(NumericUnchecked.ToByte('/'), UriEscapeComponent.Path)).IsTrue();
}
testcase Given_uri_escape_append_hex_escape_When_executed_Then_percent_encoded_is_emitted()
{
    var buffer = FVec.WithCapacity <byte >(8usize);
    UriEscape.AppendHexEscape(ref buffer, NumericUnchecked.ToByte(' '));
    let rendered = Utf8String.FromSpan(FVec.AsReadOnlySpan <byte >(in buffer));
    FVecIntrinsics.chic_rt_vec_drop(ref buffer);
    Assert.That(rendered).IsEqualTo("%20");
}
testcase Given_uri_idn_try_get_ascii_host_When_executed_Then_roundtrips()
{
    let ok = UriIdn.TryGetAsciiHost("bücher.example".AsUtf8Span(), out var asciiHost, out var error);
    Assert.That(ok).IsTrue();
    Assert.That(error.Length).IsEqualTo(0);
    Assert.That(asciiHost).IsEqualTo("bücher.example");
}
testcase Given_uri_find_scheme_When_executed_Then_returns_expected_index()
{
    Assert.That(Uri.FindScheme("http://example.com".AsUtf8Span())).IsEqualTo(4);
    Assert.That(Uri.FindScheme("/relative/path".AsUtf8Span())).IsEqualTo(- 1);
}
testcase Given_uri_ascii_helpers_When_executed_Then_return_expected_results()
{
    Assert.That(Uri.IsAlpha(NumericUnchecked.ToByte('A'))).IsTrue();
    Assert.That(Uri.IsAlpha(NumericUnchecked.ToByte('9'))).IsFalse();
    Assert.That(Uri.IsDigit(NumericUnchecked.ToByte('9'))).IsTrue();
    Assert.That(Uri.IsDigit(NumericUnchecked.ToByte('a'))).IsFalse();
    Assert.That(Uri.IsSchemeChar(NumericUnchecked.ToByte('+'))).IsTrue();
    Assert.That(Uri.IsUnreservedAscii(NumericUnchecked.ToByte('_'))).IsTrue();
    Assert.That(Uri.IsSubDelimAscii(NumericUnchecked.ToByte('!'))).IsTrue();
}
testcase Given_uri_equals_ascii_is_case_insensitive_When_executed_Then_true_for_matching_ascii()
{
    Assert.That(Uri.EqualsAscii("HTTP", "http")).IsTrue();
    Assert.That(Uri.EqualsAscii("http", "https")).IsFalse();
}
testcase Given_uri_to_lower_ascii_When_executed_Then_lowercases_ascii()
{
    Assert.That(Uri.ToLowerAscii("HTTP".AsUtf8Span())).IsEqualTo("http");
    Assert.That(Uri.ToLowerAsciiChar('Z')).IsEqualTo('z');
    Assert.That(Uri.ToLowerAsciiChar('z')).IsEqualTo('z');
}
testcase Given_uri_concat_and_format_component_When_executed_Then_results_match()
{
    Assert.That(Uri.Concat2("?", "x=1")).IsEqualTo("?x=1");
    Assert.That(Uri.Concat3("a", "b", "c")).IsEqualTo("abc");
    Assert.That(Uri.FormatComponent("%2F", UriFormat.Unescaped, UriEscapeComponent.Path)).IsEqualTo("/");
    Assert.That(Uri.FormatComponent("%2F", UriFormat.SafeUnescaped, UriEscapeComponent.Path)).IsEqualTo("%2F");
    Assert.That(Uri.FormatComponent("/", UriFormat.UriEscaped, UriEscapeComponent.Path)).IsEqualTo("/");
}
testcase Given_uri_validate_component_percent_encoding_When_executed_Then_user_escaped_set()
{
    let ok = Uri.ValidateComponent("a%20b".AsUtf8Span(), true, true, out var escaped);
    Assert.That(ok).IsTrue();
    Assert.That(escaped).IsTrue();
    let bad = Uri.ValidateComponent("a%ZZ".AsUtf8Span(), true, true, out var ignored);
    Assert.That(bad).IsFalse();
}
testcase Given_uri_validate_user_info_When_executed_Then_colon_and_unreserved_allowed()
{
    let ok = Uri.ValidateUserInfo("user:pass".AsUtf8Span(), out var escaped);
    Assert.That(ok).IsTrue();
    Assert.That(escaped).IsFalse();
    let okEscaped = Uri.ValidateUserInfo("user%20pass:".AsUtf8Span(), out var escaped2);
    Assert.That(okEscaped).IsTrue();
    Assert.That(escaped2).IsTrue();
}
testcase Given_uri_parse_port_When_executed_Then_returns_expected_status()
{
    Assert.That(Uri.ParsePort("0".AsUtf8Span(), out var port0)).IsTrue();
    Assert.That(port0).IsEqualTo(0);
    Assert.That(Uri.ParsePort("65535".AsUtf8Span(), out var portMax)).IsTrue();
    Assert.That(portMax).IsEqualTo(65535);
    Assert.That(Uri.ParsePort("65536".AsUtf8Span(), out var tooBig)).IsFalse();
    Assert.That(Uri.ParsePort("abc".AsUtf8Span(), out var notNumber)).IsFalse();
}
testcase Given_uri_is_valid_hosts_When_executed_Then_expected_booleans_returned()
{
    Assert.That(Uri.IsValidBasicHost("example.com".AsUtf8Span())).IsTrue();
    Assert.That(Uri.IsValidBasicHost("a..b".AsUtf8Span())).IsFalse();
    Assert.That(Uri.IsValidUnicodeHost("example.com".AsUtf8Span())).IsTrue();
    Assert.That(Uri.IsValidUnicodeHost(".bad".AsUtf8Span())).IsFalse();
}
testcase Given_uri_is_loopback_host_When_executed_Then_expected_values_returned()
{
    Assert.That(Uri.IsLoopbackHost("", UriHostNameType.Unknown, true)).IsTrue();
    Assert.That(Uri.IsLoopbackHost("127.0.0.1", UriHostNameType.IPv4, false)).IsTrue();
    Assert.That(Uri.IsLoopbackHost("::1", UriHostNameType.IPv6, false)).IsTrue();
    Assert.That(Uri.IsLoopbackHost("localhost", UriHostNameType.Dns, false)).IsTrue();
    Assert.That(Uri.IsLoopbackHost("example.com", UriHostNameType.Dns, false)).IsFalse();
}
testcase Given_uri_to_parts_and_apply_parsed_When_executed_Then_roundtrip_matches()
{
    let uri = new Uri("http://user:pass@example.com:8080/path?x=1#frag");
    let parts = uri.ToParts();
    Assert.That(parts.Scheme).IsEqualTo("http");
    Assert.That(parts.Host).IsEqualTo("example.com");
    Assert.That(parts.Port).IsEqualTo(8080);
    Assert.That(parts.Query).IsEqualTo("x=1");
    Assert.That(parts.Fragment).IsEqualTo("frag");
    let rebuilt = new Uri("http://example.com/", UriKind.Absolute);
    rebuilt.ApplyParsed(uri.OriginalString, parts);
    Assert.That(rebuilt.Host).IsEqualTo("example.com");
    Assert.That(rebuilt.Port).IsEqualTo(8080);
}
testcase Given_uri_resolve_and_build_strings_When_executed_Then_outputs_are_stable()
{
    let baseUri = new Uri("http://example.com/a/b/c");
    let ok = Uri.TryParseCore("../d/e?z=2#f", UriKind.RelativeOrAbsolute, out var relativeParts, out var error);
    Assert.That(ok).IsTrue();
    Assert.That(error.Length).IsEqualTo(0);
    let resolved = Uri.Resolve(baseUri, relativeParts);
    let uriEscaped = resolved.BuildUriString(UriFormat.UriEscaped);
    Assert.That(uriEscaped).Contains("/a/d/e");
    let rebuiltOriginal = Uri.BuildOriginalString(resolved.ToParts());
    Assert.That(rebuiltOriginal).Contains("http");
}
