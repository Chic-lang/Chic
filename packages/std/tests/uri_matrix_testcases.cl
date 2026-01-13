namespace Std;
import Std.Core;
import Std.Testing;
testcase Given_uri_matrix_When_executed_Then_try_create_and_common_apis_work()
{
    let samples = new string[12];
    samples[0] = "http://example.com";
    samples[1] = "https://example.com:443/path";
    samples[2] = "ftp://user:pass@example.com:21/dir";
    samples[3] = "ws://example.com/";
    samples[4] = "wss://example.com/";
    samples[5] = "file:///tmp/example.txt";
    samples[6] = "file://server/share/file.txt";
    samples[7] = "mailto:user@example.com";
    samples[8] = "/relative/path";
    samples[9] = "../up";
    samples[10] = "?q=1";
    samples[11] = "http://[2001:db8::1]/";
    var index = 0;
    while (index <samples.Length)
    {
        let text = samples[index];
        let ok = Uri.TryCreate(text, UriKind.RelativeOrAbsolute, out var uri);
        Assert.That(ok).IsTrue();
        Assert.That(uri).IsNotNull();
        Assert.That(uri.OriginalString.Length >0).IsTrue();
        Assert.That(uri.ToString().Length >0).IsTrue();
        let _ = uri.GetHashCode();
        if (uri.IsAbsoluteUri)
        {
            Assert.That(uri.Scheme.Length >0).IsTrue();
            Assert.That(uri.GetLeftPart(UriPartial.Scheme)).Contains(":");
            Assert.That(uri.GetComponents(UriComponents.Scheme, UriFormat.Unescaped).Length >0).IsTrue();
            let _ = uri.AbsoluteUri;
            let _ = uri.Segments;
            if (uri.IsFile)
            {
                let _ = uri.LocalPath;
            }
        }
        index += 1;
    }
}
testcase Given_uri_try_create_invalid_inputs_When_executed_Then_returns_false()
{
    let bad = new string[4];
    bad[0] = "http://[::1";
    bad[1] = "http://example.com:999999/path";
    bad[2] = "://missing-scheme";
    bad[3] = "http://exa mple.com/";
    var index = 0;
    while (index <bad.Length)
    {
        let ok = Uri.TryCreate(bad[index], UriKind.Absolute, out var uri);
        Assert.That(ok).IsFalse();
        Assert.That(uri).IsNull();
        index += 1;
    }
}
