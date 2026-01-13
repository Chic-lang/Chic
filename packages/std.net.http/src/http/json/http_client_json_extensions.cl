namespace Std.Net.Http.Json;
import Std.Async;
import Std.Net.Http;
import Std.Strings;
import Std.Span;
import Std.Text.Json;
/// <summary>HTTP + JSON extension methods aligned with System.Net.Http.Json.</summary>
public static class HttpClientJsonExtensions
{
    private static JsonSerializerOptions EnsureOptions <T >(JsonSerializerOptions ?options, JsonTypeInfo <T >?typeInfo) {
        if (options != null)
        {
            return options;
        }
        if (typeInfo != null)
        {
            return typeInfo.Options;
        }
        return new JsonSerializerOptions();
    }
    private static T ReadJson <T >(HttpResponseMessage response, JsonSerializerOptions ?options, JsonTypeInfo <T >?info) {
        if (response.Content == null)
        {
            throw new HttpRequestException("Response content missing");
        }
        let text = response.Content.ReadAsString();
        let opts = EnsureOptions(options, info);
        return JsonSerializer.Deserialize <T >(text, opts);
    }
    private static HttpContent WrapJson <T >(T value, JsonSerializerOptions ?options, JsonTypeInfo <T >?info) {
        let opts = EnsureOptions(options, info);
        let json = JsonSerializer.Serialize(value, opts);
        var content = new StringContent(json);
        content.Headers.Set("Content-Type", "application/json; charset=utf-8");
        return content;
    }
    // GET
    public static Task <TValue >GetFromJsonAsync <TValue >(HttpClient client, string uri, CancellationToken cancellationToken) {
        let responseTask = client.GetAsync(uri, HttpCompletionOption.ResponseContentRead, cancellationToken);
        let response = Task <HttpResponseMessage >.Scope(responseTask);
        let value = ReadJson(response, null, null);
        return TaskRuntime.FromResult(value);
    }
    public static Task <TValue >GetFromJsonAsync <TValue >(HttpClient client, Std.Uri uri, CancellationToken cancellationToken) {
        let responseTask = client.GetAsync(uri, HttpCompletionOption.ResponseContentRead, cancellationToken);
        let response = Task <HttpResponseMessage >.Scope(responseTask);
        let value = ReadJson(response, null, null);
        return TaskRuntime.FromResult(value);
    }
    public static Task <TValue >GetFromJsonAsync <TValue >(HttpClient client, string uri, JsonSerializerOptions options,
    CancellationToken cancellationToken) {
        let responseTask = client.GetAsync(uri, HttpCompletionOption.ResponseContentRead, cancellationToken);
        let response = Task <HttpResponseMessage >.Scope(responseTask);
        let value = ReadJson(response, options, null);
        return TaskRuntime.FromResult(value);
    }
    public static Task <TValue >GetFromJsonAsync <TValue >(HttpClient client, Std.Uri uri, JsonSerializerOptions options,
    CancellationToken cancellationToken) {
        let responseTask = client.GetAsync(uri, HttpCompletionOption.ResponseContentRead, cancellationToken);
        let response = Task <HttpResponseMessage >.Scope(responseTask);
        let value = ReadJson(response, options, null);
        return TaskRuntime.FromResult(value);
    }
    public static Task <TValue >GetFromJsonAsync <TValue >(HttpClient client, string uri, JsonTypeInfo <TValue >typeInfo,
    CancellationToken cancellationToken) {
        let responseTask = client.GetAsync(uri, HttpCompletionOption.ResponseContentRead, cancellationToken);
        let response = Task <HttpResponseMessage >.Scope(responseTask);
        let value = ReadJson(response, null, typeInfo);
        return TaskRuntime.FromResult(value);
    }
    public static Task <TValue >GetFromJsonAsync <TValue >(HttpClient client, Std.Uri uri, JsonTypeInfo <TValue >typeInfo,
    CancellationToken cancellationToken) {
        let responseTask = client.GetAsync(uri, HttpCompletionOption.ResponseContentRead, cancellationToken);
        let response = Task <HttpResponseMessage >.Scope(responseTask);
        let value = ReadJson(response, null, typeInfo);
        return TaskRuntime.FromResult(value);
    }
    public static Task <object >GetFromJsonAsync(HttpClient client, string uri, Std.Type returnType, JsonSerializerOptions options,
    CancellationToken cancellationToken) {
        let responseTask = client.GetAsync(uri, HttpCompletionOption.ResponseContentRead, cancellationToken);
        let response = Task <HttpResponseMessage >.Scope(responseTask);
        if (response.Content == null)
        {
            throw new HttpRequestException("Response content missing");
        }
        let text = response.Content.ReadAsString();
        return TaskRuntime.FromResult(JsonSerializer.Deserialize(text, returnType, options));
    }
    public static Task <object >GetFromJsonAsync(HttpClient client, Std.Uri uri, Std.Type returnType, JsonSerializerOptions options,
    CancellationToken cancellationToken) {
        let responseTask = client.GetAsync(uri, HttpCompletionOption.ResponseContentRead, cancellationToken);
        let response = Task <HttpResponseMessage >.Scope(responseTask);
        if (response.Content == null)
        {
            throw new HttpRequestException("Response content missing");
        }
        let text = response.Content.ReadAsString();
        return TaskRuntime.FromResult(JsonSerializer.Deserialize(text, returnType, options));
    }
    // DELETE
    public static Task <TValue >DeleteFromJsonAsync <TValue >(HttpClient client, string uri, CancellationToken cancellationToken) {
        let responseTask = client.DeleteAsync(uri, cancellationToken);
        let response = Task <HttpResponseMessage >.Scope(responseTask);
        let value = ReadJson(response, null, null);
        return TaskRuntime.FromResult(value);
    }
    public static Task <TValue >DeleteFromJsonAsync <TValue >(HttpClient client, Std.Uri uri, CancellationToken cancellationToken) {
        let responseTask = client.DeleteAsync(uri, cancellationToken);
        let response = Task <HttpResponseMessage >.Scope(responseTask);
        let value = ReadJson(response, null, null);
        return TaskRuntime.FromResult(value);
    }
    public static Task <TValue >DeleteFromJsonAsync <TValue >(HttpClient client, string uri, JsonSerializerOptions options,
    CancellationToken cancellationToken) {
        let responseTask = client.DeleteAsync(uri, cancellationToken);
        let response = Task <HttpResponseMessage >.Scope(responseTask);
        let value = ReadJson(response, options, null);
        return TaskRuntime.FromResult(value);
    }
    public static Task <TValue >DeleteFromJsonAsync <TValue >(HttpClient client, Std.Uri uri, JsonSerializerOptions options,
    CancellationToken cancellationToken) {
        let responseTask = client.DeleteAsync(uri, cancellationToken);
        let response = Task <HttpResponseMessage >.Scope(responseTask);
        let value = ReadJson(response, options, null);
        return TaskRuntime.FromResult(value);
    }
    public static Task <TValue >DeleteFromJsonAsync <TValue >(HttpClient client, string uri, JsonTypeInfo <TValue >typeInfo,
    CancellationToken cancellationToken) {
        let responseTask = client.DeleteAsync(uri, cancellationToken);
        let response = Task <HttpResponseMessage >.Scope(responseTask);
        let value = ReadJson(response, null, typeInfo);
        return TaskRuntime.FromResult(value);
    }
    public static Task <object >DeleteFromJsonAsync(HttpClient client, string uri, Std.Type returnType, JsonSerializerOptions options,
    CancellationToken cancellationToken) {
        let responseTask = client.DeleteAsync(uri, cancellationToken);
        let response = Task <HttpResponseMessage >.Scope(responseTask);
        if (response.Content == null)
        {
            throw new HttpRequestException("Response content missing");
        }
        let text = response.Content.ReadAsString();
        return TaskRuntime.FromResult(JsonSerializer.Deserialize(text, returnType, options));
    }
    public static Task <object >DeleteFromJsonAsync(HttpClient client, Std.Uri uri, Std.Type returnType, JsonSerializerOptions options,
    CancellationToken cancellationToken) {
        let responseTask = client.DeleteAsync(uri, cancellationToken);
        let response = Task <HttpResponseMessage >.Scope(responseTask);
        if (response.Content == null)
        {
            throw new HttpRequestException("Response content missing");
        }
        let text = response.Content.ReadAsString();
        return TaskRuntime.FromResult(JsonSerializer.Deserialize(text, returnType, options));
    }
    public static Task <TValue >DeleteFromJsonAsync <TValue >(HttpClient client, Std.Uri uri, JsonTypeInfo <TValue >typeInfo,
    CancellationToken cancellationToken) {
        let responseTask = client.DeleteAsync(uri, cancellationToken);
        let response = Task <HttpResponseMessage >.Scope(responseTask);
        let value = ReadJson(response, null, typeInfo);
        return TaskRuntime.FromResult(value);
    }
    // POST
    public static Task <HttpResponseMessage >PostAsJsonAsync <TValue >(HttpClient client, string uri, TValue value, CancellationToken cancellationToken) {
        return client.PostAsync(uri, WrapJson(value, null, null), cancellationToken);
    }
    public static Task <HttpResponseMessage >PostAsJsonAsync <TValue >(HttpClient client, Std.Uri uri, TValue value, CancellationToken cancellationToken) {
        return client.PostAsync(uri, WrapJson(value, null, null), cancellationToken);
    }
    public static Task <HttpResponseMessage >PostAsJsonAsync <TValue >(HttpClient client, string uri, TValue value, JsonSerializerOptions options,
    CancellationToken cancellationToken) {
        return client.PostAsync(uri, WrapJson(value, options, null), cancellationToken);
    }
    public static Task <HttpResponseMessage >PostAsJsonAsync <TValue >(HttpClient client, Std.Uri uri, TValue value, JsonSerializerOptions options,
    CancellationToken cancellationToken) {
        return client.PostAsync(uri, WrapJson(value, options, null), cancellationToken);
    }
    public static Task <HttpResponseMessage >PostAsJsonAsync <TValue >(HttpClient client, string uri, TValue value, JsonTypeInfo <TValue >typeInfo,
    CancellationToken cancellationToken) {
        return client.PostAsync(uri, WrapJson(value, null, typeInfo), cancellationToken);
    }
    public static Task <HttpResponseMessage >PostAsJsonAsync <TValue >(HttpClient client, Std.Uri uri, TValue value, JsonTypeInfo <TValue >typeInfo,
    CancellationToken cancellationToken) {
        return client.PostAsync(uri, WrapJson(value, null, typeInfo), cancellationToken);
    }
    // PUT
    public static Task <HttpResponseMessage >PutAsJsonAsync <TValue >(HttpClient client, string uri, TValue value, CancellationToken cancellationToken) {
        return client.PutAsync(uri, WrapJson(value, null, null), cancellationToken);
    }
    public static Task <HttpResponseMessage >PutAsJsonAsync <TValue >(HttpClient client, Std.Uri uri, TValue value, CancellationToken cancellationToken) {
        return client.PutAsync(uri, WrapJson(value, null, null), cancellationToken);
    }
    public static Task <HttpResponseMessage >PutAsJsonAsync <TValue >(HttpClient client, string uri, TValue value, JsonSerializerOptions options,
    CancellationToken cancellationToken) {
        return client.PutAsync(uri, WrapJson(value, options, null), cancellationToken);
    }
    public static Task <HttpResponseMessage >PutAsJsonAsync <TValue >(HttpClient client, Std.Uri uri, TValue value, JsonSerializerOptions options,
    CancellationToken cancellationToken) {
        return client.PutAsync(uri, WrapJson(value, options, null), cancellationToken);
    }
    public static Task <HttpResponseMessage >PutAsJsonAsync <TValue >(HttpClient client, string uri, TValue value, JsonTypeInfo <TValue >typeInfo,
    CancellationToken cancellationToken) {
        return client.PutAsync(uri, WrapJson(value, null, typeInfo), cancellationToken);
    }
    public static Task <HttpResponseMessage >PutAsJsonAsync <TValue >(HttpClient client, Std.Uri uri, TValue value, JsonTypeInfo <TValue >typeInfo,
    CancellationToken cancellationToken) {
        return client.PutAsync(uri, WrapJson(value, null, typeInfo), cancellationToken);
    }
    // PATCH
    public static Task <HttpResponseMessage >PatchAsJsonAsync <TValue >(HttpClient client, string uri, TValue value, CancellationToken cancellationToken) {
        return client.PatchAsync(uri, WrapJson(value, null, null), cancellationToken);
    }
    public static Task <HttpResponseMessage >PatchAsJsonAsync <TValue >(HttpClient client, Std.Uri uri, TValue value, CancellationToken cancellationToken) {
        return client.PatchAsync(uri, WrapJson(value, null, null), cancellationToken);
    }
    public static Task <HttpResponseMessage >PatchAsJsonAsync <TValue >(HttpClient client, string uri, TValue value, JsonSerializerOptions options,
    CancellationToken cancellationToken) {
        return client.PatchAsync(uri, WrapJson(value, options, null), cancellationToken);
    }
    public static Task <HttpResponseMessage >PatchAsJsonAsync <TValue >(HttpClient client, Std.Uri uri, TValue value, JsonSerializerOptions options,
    CancellationToken cancellationToken) {
        return client.PatchAsync(uri, WrapJson(value, options, null), cancellationToken);
    }
    public static Task <HttpResponseMessage >PatchAsJsonAsync <TValue >(HttpClient client, string uri, TValue value, JsonTypeInfo <TValue >typeInfo,
    CancellationToken cancellationToken) {
        return client.PatchAsync(uri, WrapJson(value, null, typeInfo), cancellationToken);
    }
    public static Task <HttpResponseMessage >PatchAsJsonAsync <TValue >(HttpClient client, Std.Uri uri, TValue value, JsonTypeInfo <TValue >typeInfo,
    CancellationToken cancellationToken) {
        return client.PatchAsync(uri, WrapJson(value, null, typeInfo), cancellationToken);
    }
}
