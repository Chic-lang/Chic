namespace Std
{
    import Std.Numeric;
    import Std.Strings;
    import Std.Runtime.Collections;
    import Foundation.Collections;
    import FVec = Foundation.Collections.Vec;
    import FVecIntrinsics = Foundation.Collections.VecIntrinsics;
    public sealed class UriBuilder
    {
        private string _scheme;
        private string _host;
        private int _port;
        private string _path;
        private string _query;
        private string _fragment;
        private string _userName;
        private string _password;
        public init() {
            _scheme = "http";
            _host = "localhost";
            _port = - 1;
            _path = "/";
            _query = Std.Runtime.StringRuntime.Create();
            _fragment = Std.Runtime.StringRuntime.Create();
            _userName = Std.Runtime.StringRuntime.Create();
            _password = Std.Runtime.StringRuntime.Create();
        }
        public init(string uri) {
            if (uri == null)
            {
                throw new ArgumentNullException("uri");
            }
            if (!Uri.TryCreate (uri, UriKind.Absolute, out var parsed)) {
                let fallback = Concat2("http://", uri);
                if (!Uri.TryCreate (fallback, UriKind.Absolute, out parsed)) {
                    throw new UriFormatException("Invalid URI");
                }
            }
            InitFromUri(parsed);
        }
        public init(Uri uri) {
            if (uri == null)
            {
                throw new ArgumentNullException("uri");
            }
            InitFromUri(uri);
        }
        public init(string schemeName, string hostName) {
            if (schemeName == null)
            {
                throw new ArgumentNullException("schemeName");
            }
            if (hostName == null)
            {
                throw new ArgumentNullException("hostName");
            }
            _scheme = ToLowerAscii(schemeName);
            _host = hostName;
            _port = - 1;
            _path = "/";
            _query = Std.Runtime.StringRuntime.Create();
            _fragment = Std.Runtime.StringRuntime.Create();
            _userName = Std.Runtime.StringRuntime.Create();
            _password = Std.Runtime.StringRuntime.Create();
        }
        public init(string scheme, string host, int portNumber) {
            if (scheme == null)
            {
                throw new ArgumentNullException("scheme");
            }
            if (host == null)
            {
                throw new ArgumentNullException("host");
            }
            _scheme = ToLowerAscii(scheme);
            _host = host;
            _port = portNumber;
            _path = "/";
            _query = Std.Runtime.StringRuntime.Create();
            _fragment = Std.Runtime.StringRuntime.Create();
            _userName = Std.Runtime.StringRuntime.Create();
            _password = Std.Runtime.StringRuntime.Create();
        }
        public string Scheme {
            get {
                return _scheme;
            }
            set {
                if (value == null || value.Length == 0)
                {
                    throw new ArgumentNullException("Scheme");
                }
                _scheme = ToLowerAscii(value);
            }
        }
        public string Host {
            get {
                return _host;
            }
            set {
                if (value == null)
                {
                    throw new ArgumentNullException("Host");
                }
                _host = value;
            }
        }
        public int Port {
            get {
                return _port;
            }
            set {
                _port = value;
            }
        }
        public string Path {
            get {
                return _path;
            }
            set {
                if (value == null)
                {
                    _path = Std.Runtime.StringRuntime.Create();
                    return;
                }
                _path = value;
            }
        }
        public string Query {
            get {
                if (_query == null || _query.Length == 0)
                {
                    return Std.Runtime.StringRuntime.Create();
                }
                return Concat2("?", _query);
            }
            set {
                if (value == null)
                {
                    _query = Std.Runtime.StringRuntime.Create();
                    return;
                }
                _query = StripPrefix(value, '?');
            }
        }
        public string Fragment {
            get {
                if (_fragment == null || _fragment.Length == 0)
                {
                    return Std.Runtime.StringRuntime.Create();
                }
                return Concat2("#", _fragment);
            }
            set {
                if (value == null)
                {
                    _fragment = Std.Runtime.StringRuntime.Create();
                    return;
                }
                _fragment = StripPrefix(value, '#');
            }
        }
        public string UserName {
            get {
                return _userName;
            }
            set {
                if (value == null)
                {
                    _userName = Std.Runtime.StringRuntime.Create();
                    return;
                }
                _userName = value;
            }
        }
        public string Password {
            get {
                return _password;
            }
            set {
                if (value == null)
                {
                    _password = Std.Runtime.StringRuntime.Create();
                    return;
                }
                _password = value;
            }
        }
        public Uri Uri {
            get {
                let built = BuildUriString();
                return new Uri(built, UriKind.Absolute);
            }
        }
        private void InitFromUri(Uri uri) {
            if (uri == null)
            {
                _scheme = "http";
                _host = "localhost";
                _port = - 1;
                _path = "/";
                _query = Std.Runtime.StringRuntime.Create();
                _fragment = Std.Runtime.StringRuntime.Create();
                _userName = Std.Runtime.StringRuntime.Create();
                _password = Std.Runtime.StringRuntime.Create();
                return;
            }
            _scheme = uri.Scheme;
            _host = uri.Host;
            _port = uri.IsDefaultPort ?- 1 : uri.Port;
            _path = uri.AbsolutePath;
            _query = StripPrefix(uri.Query, '?');
            _fragment = StripPrefix(uri.Fragment, '#');
            _userName = Std.Runtime.StringRuntime.Create();
            _password = Std.Runtime.StringRuntime.Create();
        }
        private string BuildUriString() {
            var result = Std.Runtime.StringRuntime.FromStr("http");
            if (_scheme != null)
            {
                result = _scheme;
            }
            let schemeDelimiter = Std.Runtime.StringRuntime.FromStr("://");
            let colon = Std.Runtime.StringRuntime.FromStr(":");
            let atSymbol = Std.Runtime.StringRuntime.FromStr("@");
            let queryPrefix = Std.Runtime.StringRuntime.FromStr("?");
            let fragmentPrefix = Std.Runtime.StringRuntime.FromStr("#");
            result = Concat2(result, schemeDelimiter);
            if (_userName != null && _userName.Length >0)
            {
                result = Concat2(result, _userName);
                if (_password != null && _password.Length >0)
                {
                    result = Concat2(result, colon);
                    result = Concat2(result, _password);
                }
                result = Concat2(result, atSymbol);
            }
            if (_host == null)
            {
                result = Concat2(result, Std.Runtime.StringRuntime.Create());
            }
            else
            {
                result = Concat2(result, _host);
            }
            if (_port >= 0)
            {
                result = Concat2(result, colon);
                result = Concat2(result, _port.ToString());
            }
            var path = Std.Runtime.StringRuntime.FromStr("/");
            if (_path != null && _path.Length >0)
            {
                path = _path;
            }
            if (path.Length == 0 || path[0] != '/')
            {
                path = Concat2(Std.Runtime.StringRuntime.FromStr("/"), path);
            }
            result = Concat2(result, path);
            if (_query != null && _query.Length >0)
            {
                result = Concat2(result, queryPrefix);
                result = Concat2(result, _query);
            }
            if (_fragment != null && _fragment.Length >0)
            {
                result = Concat2(result, fragmentPrefix);
                result = Concat2(result, _fragment);
            }
            return result;
        }
        private static void AppendString(ref VecPtr buffer, string value) {
            if (value == null || value.Length == 0)
            {
                return;
            }
            let span = value.AsUtf8Span();
            var index = 0usize;
            while (index <span.Length)
            {
                FVec.Push <byte >(ref buffer, span[index]);
                index += 1;
            }
        }
        private static string Concat2(string left, string right) {
            var buffer = FVec.WithCapacity <byte >(NumericUnchecked.ToUSize((left == null ?0 : left.Length) + (right == null ?0 : right.Length)));
            AppendString(ref buffer, left);
            AppendString(ref buffer, right);
            let result = Utf8String.FromSpan(Foundation.Collections.Vec.AsReadOnlySpan <byte >(in buffer));
            FVecIntrinsics.chic_rt_vec_drop(ref buffer);
            return result;
        }
        private static string StripPrefix(string value, char prefix) {
            if (value == null || value.Length == 0)
            {
                return Std.Runtime.StringRuntime.Create();
            }
            if (value[0] == prefix)
            {
                if (value.Length == 1)
                {
                    return Std.Runtime.StringRuntime.Create();
                }
                return SliceString(value, 1, value.Length - 1);
            }
            return value;
        }
        private static int IndexOf(string value, char needle) {
            if (value == null)
            {
                return - 1;
            }
            var index = 0;
            while (index <value.Length)
            {
                if (value[index] == needle)
                {
                    return index;
                }
                index += 1;
            }
            return - 1;
        }
        private static string SliceString(string value, int start, int length) {
            if (value == null || length <= 0)
            {
                return Std.Runtime.StringRuntime.Create();
            }
            let span = value.AsUtf8Span();
            let slice = span.Slice(NumericUnchecked.ToUSize(start), NumericUnchecked.ToUSize(length));
            return Utf8String.FromSpan(slice);
        }
        private static string ToLowerAscii(string value) {
            if (value == null || value.Length == 0)
            {
                return Std.Runtime.StringRuntime.Create();
            }
            let span = value.AsUtf8Span();
            var buffer = FVec.WithCapacity <byte >(span.Length);
            var index = 0usize;
            while (index <span.Length)
            {
                var current = span[index];
                if (current >= NumericUnchecked.ToByte ('A') && current <= NumericUnchecked.ToByte ('Z'))
                {
                    current = NumericUnchecked.ToByte(NumericUnchecked.ToInt32(current) + 32);
                }
                FVec.Push <byte >(ref buffer, current);
                index += 1;
            }
            let result = Utf8String.FromSpan(Foundation.Collections.Vec.AsReadOnlySpan <byte >(in buffer));
            FVecIntrinsics.chic_rt_vec_drop(ref buffer);
            return result;
        }
    }
}
