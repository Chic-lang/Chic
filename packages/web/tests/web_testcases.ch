namespace Chic.Web;
import Std.IO;
import Std.Testing;
testcase Given_query_collection_parse_reads_value_When_executed_Then_query_collection_parse_reads_value()
{
    let query = QueryCollection.Parse("?a=1&b=two");
    Assert.That(query.GetValueOrDefault("b", "")).IsEqualTo("two");
}
testcase Given_query_collection_parse_missing_value_empty_When_executed_Then_query_collection_parse_missing_value_empty()
{
    let query = QueryCollection.Parse("flag");
    Assert.That(query.GetValueOrDefault("flag", "fallback")).IsEqualTo("");
}
testcase Given_query_collection_try_get_missing_returns_false_When_executed_Then_query_collection_try_get_missing_returns_false()
{
    let query = QueryCollection.Parse("a=1");
    let ok = query.TryGetValue("missing", out var value);
    let _ = value;
    Assert.That(ok).IsFalse();
}
testcase Given_route_values_set_null_name_no_entry_When_executed_Then_route_values_set_null_name_no_entry()
{
    let values = new RouteValues();
    values.Set(null, "value");
    let ok = values.TryGetValue("value", out var output);
    let _ = output;
    Assert.That(ok).IsFalse();
}
testcase Given_route_template_try_match_literal_success_When_executed_Then_route_template_try_match_literal_success()
{
    let template = new RouteTemplate("/users/list");
    let ok = template.TryMatch("/users/list", out var values);
    let _ = values;
    Assert.That(ok).IsTrue();
}
testcase Given_route_template_try_match_parameter_captures_id_When_executed_Then_route_template_try_match_parameter_captures_id()
{
    let template = new RouteTemplate("/users/{id}");
    let ok = template.TryMatch("/users/42", out var values);
    let captured = values.TryGetValue("id", out var id);
    let matches = ok && captured && id == "42";
    Assert.That(matches).IsTrue();
}
testcase Given_http_request_defaults_method_empty_When_executed_Then_http_request_defaults_method_empty()
{
    let request = new HttpRequest(null, null, null, null, null);
    Assert.That(request.Method.Length).IsEqualTo(0);
}
testcase Given_http_request_defaults_path_root_When_executed_Then_http_request_defaults_path_root()
{
    let request = new HttpRequest("GET", null, "", null, null);
    Assert.That(request.Path).IsEqualTo("/");
}
testcase Given_http_request_query_parsed_When_executed_Then_http_request_query_parsed()
{
    let request = new HttpRequest("GET", "/path", "?a=1", null, null);
    let value = request.Query.GetValueOrDefault("a", "");
    Assert.That(value).IsEqualTo("1");
}
testcase Given_http_response_default_status_code_When_executed_Then_http_response_default_status_code()
{
    let response = new HttpResponse();
    Assert.That(response.StatusCode).IsEqualTo(200);
}
testcase Given_http_response_content_length_sets_flag_When_executed_Then_http_response_content_length_sets_flag()
{
    let response = new HttpResponse();
    response.ContentLength = 5;
    Assert.That(response.HasContentLength).IsTrue();
}
testcase Given_http_response_write_string_writes_length_When_executed_Then_http_response_write_string_writes_length()
{
    let response = new HttpResponse();
    let _ = response.WriteStringAsync("hi");
    Assert.That(response.BodyStream.Length).IsEqualTo(2);
}
testcase Given_http_response_mark_started_sets_flag_When_executed_Then_http_response_mark_started_sets_flag()
{
    let response = new HttpResponse();
    response.MarkStarted();
    Assert.That(response.HasStarted).IsTrue();
}
