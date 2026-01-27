namespace Std
{
    import Std.Numeric;
    import Std.Strings;
    import Std.Span;
    import Std.Collections;
    import Std.Runtime.Collections;
    import Std.Core;
    import Foundation.Collections;
    import FVec = Foundation.Collections.Vec;
    import FVecIntrinsics = Foundation.Collections.VecIntrinsics;
    public class UriFormatException : FormatException
    {
        public init() : super() {
        }
        public init(str message) : super(Std.Runtime.StringRuntime.FromStr(message)) {
        }
        public init(string message) : super(message) {
        }
    }
    internal struct UriParts
    {
        public bool IsAbsolute;
        public bool HasAuthority;
        public string Scheme;
        public string UserInfo;
        public string Host;
        public string IdnHost;
        public UriHostNameType HostNameType;
        public int Port;
        public bool PortSpecified;
        public string Path;
        public string Query;
        public string Fragment;
        public bool UserEscaped;
        public bool IsFile;
        public bool IsUnc;
        public bool IsLoopback;
    }
    public sealed class Uri
    {
        public const string SchemeDelimiter = "://";
        public const string UriSchemeHttp = "http";
        public const string UriSchemeHttps = "https";
        public const string UriSchemeFtp = "ftp";
        public const string UriSchemeFile = "file";
        public const string UriSchemeMailto = "mailto";
        public const string UriSchemeWs = "ws";
        public const string UriSchemeWss = "wss";
        private string _originalString;
        private bool _isAbsolute;
        private bool _hasAuthority;
        private string _scheme;
        private string _userInfo;
        private string _host;
        private string _idnHost;
        private UriHostNameType _hostNameType;
        private int _port;
        private bool _portSpecified;
        private string _path;
        private string _query;
        private string _fragment;
        private bool _userEscaped;
        private bool _isFile;
        private bool _isUnc;
        private bool _isLoopback;
        private string ?_cachedAbsoluteUri;
        private string ?_cachedToString;
        private string ?_cachedAbsolutePath;
        private string ?_cachedPathAndQuery;
        private string[] ?_segments;
        private bool _segmentsComputed;
        public init(string uriString) {
            if (uriString == null)
            {
                throw new ArgumentNullException("uriString");
            }
            if (! TryParseCore (uriString, UriKind.RelativeOrAbsolute, out var parsed, out var error)) {
                throw new UriFormatException(error);
            }
            ApplyParsed(uriString, parsed);
        }
        public init(string uriString, UriKind uriKind) {
            if (uriString == null)
            {
                throw new ArgumentNullException("uriString");
            }
            if (! TryParseCore (uriString, uriKind, out var parsed, out var error)) {
                throw new UriFormatException(error);
            }
            ApplyParsed(uriString, parsed);
        }
        public init(Uri baseUri, string relativeUri) {
            if (baseUri == null)
            {
                throw new ArgumentNullException("baseUri");
            }
            if (relativeUri == null)
            {
                throw new ArgumentNullException("relativeUri");
            }
            if (! baseUri.IsAbsoluteUri)
            {
                throw new UriFormatException("Base URI must be absolute");
            }
            if (! TryParseCore (relativeUri, UriKind.RelativeOrAbsolute, out var parsed, out var error)) {
                throw new UriFormatException(error);
            }
            if (parsed.IsAbsolute)
            {
                ApplyParsed(relativeUri, parsed);
                return;
            }
            var resolved = Resolve(baseUri, parsed);
            ApplyParsed(resolved._originalString, resolved.ToParts());
        }
        public init(Uri baseUri, Uri relativeUri) {
            if (baseUri == null)
            {
                throw new ArgumentNullException("baseUri");
            }
            if (relativeUri == null)
            {
                throw new ArgumentNullException("relativeUri");
            }
            if (! baseUri.IsAbsoluteUri)
            {
                throw new UriFormatException("Base URI must be absolute");
            }
            if (relativeUri.IsAbsoluteUri)
            {
                ApplyParsed(relativeUri._originalString, relativeUri.ToParts());
                return;
            }
            var resolved = Resolve(baseUri, relativeUri.ToParts());
            ApplyParsed(resolved._originalString, resolved.ToParts());
        }
        public static bool TryCreate(string uriString, UriKind uriKind, out Uri ?result) {
            if (uriString == null)
            {
                result = null;
                return false;
            }
            if (! TryParseCore (uriString, uriKind, out var parsed, out var parseStatus)) {
                result = null;
                return false;
            }
            var uri = new Uri(uriString, UriKind.RelativeOrAbsolute);
            uri.ApplyParsed(uriString, parsed);
            result = uri;
            return true;
        }
        public static bool TryCreate(Uri baseUri, string relativeUri, out Uri ?result) {
            result = null;
            if (baseUri == null || relativeUri == null)
            {
                return false;
            }
            if (! baseUri.IsAbsoluteUri)
            {
                return false;
            }
            if (! TryParseCore (relativeUri, UriKind.RelativeOrAbsolute, out var parsed, out var parseStatus)) {
                return false;
            }
            if (parsed.IsAbsolute)
            {
                var absolute = new Uri(relativeUri, UriKind.RelativeOrAbsolute);
                absolute.ApplyParsed(relativeUri, parsed);
                result = absolute;
                return true;
            }
            result = Resolve(baseUri, parsed);
            return true;
        }
        public static bool TryCreate(Uri baseUri, Uri relativeUri, out Uri ?result) {
            result = null;
            if (baseUri == null || relativeUri == null)
            {
                return false;
            }
            if (! baseUri.IsAbsoluteUri)
            {
                return false;
            }
            if (relativeUri.IsAbsoluteUri)
            {
                result = relativeUri;
                return true;
            }
            result = Resolve(baseUri, relativeUri.ToParts());
            return true;
        }
        public bool IsAbsoluteUri => _isAbsolute;
        public string AbsoluteUri {
            get {
                EnsureAbsolute();
                if (_cachedAbsoluteUri != null)
                {
                    return _cachedAbsoluteUri;
                }
                let built = BuildUriString(UriFormat.UriEscaped);
                _cachedAbsoluteUri = built;
                return built;
            }
        }
        public string OriginalString => _originalString;
        public string ToString() {
            if (_cachedToString != null)
            {
                return _cachedToString;
            }
            let built = BuildUriString(UriFormat.UriEscaped);
            _cachedToString = built;
            return built;
        }
        public string Scheme {
            get {
                EnsureAbsolute();
                return _scheme;
            }
        }
        public string Authority {
            get {
                EnsureAbsolute();
                return BuildAuthority(false, true, UriFormat.UriEscaped);
            }
        }
        public string Host {
            get {
                EnsureAbsolute();
                return _host;
            }
        }
        public UriHostNameType HostNameType {
            get {
                EnsureAbsolute();
                return _hostNameType;
            }
        }
        public string DnsSafeHost {
            get {
                EnsureAbsolute();
                if (_hostNameType == UriHostNameType.IPv4 || _hostNameType == UriHostNameType.IPv6)
                {
                    return _host;
                }
                if (_idnHost != null && _idnHost.Length >0)
                {
                    return _idnHost;
                }
                return _host;
            }
        }
        public string IdnHost {
            get {
                EnsureAbsolute();
                if (_idnHost != null && _idnHost.Length >0)
                {
                    return _idnHost;
                }
                return _host;
            }
        }
        public int Port {
            get {
                EnsureAbsolute();
                if (_portSpecified)
                {
                    return _port;
                }
                return DefaultPort(_scheme);
            }
        }
        public bool IsDefaultPort {
            get {
                EnsureAbsolute();
                let defaultPort = DefaultPort(_scheme);
                if (defaultPort <0)
                {
                    return _portSpecified == false;
                }
                if (_portSpecified)
                {
                    return _port == defaultPort;
                }
                return true;
            }
        }
        public string AbsolutePath {
            get {
                if (_cachedAbsolutePath != null)
                {
                    return _cachedAbsolutePath;
                }
                let path = BuildPath(UriFormat.UriEscaped);
                _cachedAbsolutePath = path;
                return path;
            }
        }
        public string PathAndQuery {
            get {
                if (_cachedPathAndQuery != null)
                {
                    return _cachedPathAndQuery;
                }
                let built = BuildPathAndQuery(UriFormat.UriEscaped);
                _cachedPathAndQuery = built;
                return built;
            }
        }
        public string Query {
            get {
                if (_query == null || _query.Length == 0)
                {
                    return Std.Runtime.StringRuntime.Create();
                }
                return Concat2("?", FormatComponent(_query, UriFormat.UriEscaped, UriEscapeComponent.Query));
            }
        }
        public string Fragment {
            get {
                if (_fragment == null || _fragment.Length == 0)
                {
                    return Std.Runtime.StringRuntime.Create();
                }
                return Concat2("#", FormatComponent(_fragment, UriFormat.UriEscaped, UriEscapeComponent.Fragment));
            }
        }
        public string UserInfo {
            get {
                EnsureAbsolute();
                return _userInfo;
            }
        }
        public bool UserEscaped => _userEscaped;
        public string[] Segments {
            get {
                if (_segmentsComputed && _segments != null)
                {
                    return _segments;
                }
                let segments = BuildSegments();
                _segments = segments;
                _segmentsComputed = true;
                return segments;
            }
        }
        public bool IsFile {
            get {
                EnsureAbsolute();
                return _isFile;
            }
        }
        public bool IsLoopback {
            get {
                EnsureAbsolute();
                return _isLoopback;
            }
        }
        public bool IsUnc {
            get {
                EnsureAbsolute();
                return _isUnc;
            }
        }
        public string LocalPath {
            get {
                EnsureAbsolute();
                if (! _isFile)
                {
                    throw new InvalidOperationException("LocalPath is only available for file URIs");
                }
                let path = FormatComponent(_path, UriFormat.Unescaped, UriEscapeComponent.Path);
                if (_isUnc && _host != null && _host.Length >0)
                {
                    return Concat3("//", _host, path);
                }
                return path;
            }
        }
        public string GetComponents(UriComponents components, UriFormat format) {
            if (components == UriComponents.AbsoluteUri)
            {
                if (IsAbsoluteUri)
                {
                    return AbsoluteUri;
                }
                return ToString();
            }
            if (components == UriComponents.PathAndQuery)
            {
                return BuildPathAndQuery(format);
            }
            if (components == UriComponents.Authority)
            {
                EnsureAbsolute();
                return BuildAuthority(false, true, format);
            }
            if (components == UriComponents.StrongAuthority)
            {
                EnsureAbsolute();
                return BuildAuthority(true, true, format);
            }
            var buffer = FVec.WithCapacity <byte >(32);
            if ( ( (int) components & (int) UriComponents.Scheme) != 0)
            {
                AppendString(ref buffer, _scheme);
            }
            if ( ( (int) components & (int) UriComponents.UserInfo) != 0)
            {
                AppendString(ref buffer, FormatComponent(_userInfo, format, UriEscapeComponent.UserInfo));
            }
            if ( ( (int) components & (int) UriComponents.Host) != 0)
            {
                let hostValue = (format == UriFormat.UriEscaped && _idnHost != null && _idnHost.Length >0) ?_idnHost : _host;
                AppendString(ref buffer, hostValue);
            }
            if ( ( (int) components & (int) UriComponents.Port) != 0)
            {
                let portValue = Port;
                if (portValue >= 0)
                {
                    AppendString(ref buffer, portValue.ToString());
                }
            }
            if ( ( (int) components & (int) UriComponents.Path) != 0)
            {
                AppendString(ref buffer, BuildPath(format));
            }
            if ( ( (int) components & (int) UriComponents.Query) != 0)
            {
                let q = _query;
                if (q != null && q.Length >0)
                {
                    AppendString(ref buffer, "?");
                    AppendString(ref buffer, FormatComponent(q, format, UriEscapeComponent.Query));
                }
            }
            if ( ( (int) components & (int) UriComponents.Fragment) != 0)
            {
                let f = _fragment;
                if (f != null && f.Length >0)
                {
                    AppendString(ref buffer, "#");
                    AppendString(ref buffer, FormatComponent(f, format, UriEscapeComponent.Fragment));
                }
            }
            let result = Utf8String.FromSpan(FVec.AsReadOnlySpan <byte >(in buffer));
            FVecIntrinsics.chic_rt_vec_drop(ref buffer);
            return result;
        }
        public string GetLeftPart(UriPartial part) {
            EnsureAbsolute();
            switch (part)
            {
                case UriPartial.Scheme:
                    return Concat2(_scheme, ":");
                case UriPartial.Authority:
                    return BuildLeftPartAuthority();
                case UriPartial.Path:
                    return BuildLeftPartPath();
                case UriPartial.Query:
                    return BuildLeftPartQuery();
                default :
                    return AbsoluteUri;
                }
            }
            public bool IsBaseOf(Uri uri) {
                if (uri == null)
                {
                    return false;
                }
                if (! IsAbsoluteUri || ! uri.IsAbsoluteUri)
                {
                    return false;
                }
                if (! EqualsAuthority (this, uri))
                {
                    return false;
                }
                let basePath = AbsolutePath;
                let targetPath = uri.AbsolutePath;
                if (! StartsWithOrdinal (targetPath, basePath))
                {
                    return false;
                }
                if (targetPath.Length == basePath.Length)
                {
                    return true;
                }
                if (basePath.Length == 0)
                {
                    return true;
                }
                return targetPath[basePath.Length] == '/';
            }
            public Uri MakeRelativeUri(Uri uri) {
                if (uri == null)
                {
                    throw new ArgumentNullException("uri");
                }
                EnsureAbsolute();
                if (! uri.IsAbsoluteUri || ! EqualsAuthority (this, uri))
                {
                    return uri;
                }
                let basePath = AbsolutePath;
                let targetPath = uri.AbsolutePath;
                var relativePath = BuildRelativePath(basePath, targetPath);
                let query = uri._query;
                let fragment = uri._fragment;
                var buffer = FVec.WithCapacity <byte >(relativePath.Length + 8);
                AppendString(ref buffer, relativePath);
                if (query != null && query.Length >0)
                {
                    AppendString(ref buffer, "?");
                    AppendString(ref buffer, FormatComponent(query, UriFormat.UriEscaped, UriEscapeComponent.Query));
                }
                if (fragment != null && fragment.Length >0)
                {
                    AppendString(ref buffer, "#");
                    AppendString(ref buffer, FormatComponent(fragment, UriFormat.UriEscaped, UriEscapeComponent.Fragment));
                }
                let finalText = Utf8String.FromSpan(FVec.AsReadOnlySpan <byte >(in buffer));
                FVecIntrinsics.chic_rt_vec_drop(ref buffer);
                return new Uri(finalText, UriKind.Relative);
            }
            public bool Equals(Object other) {
                return false;
            }
            public bool Equals(Uri other) {
                if (other == null)
                {
                    return false;
                }
                let left = CanonicalString();
                let right = other.CanonicalString();
                return CompareStrings(left, right, StringComparison.Ordinal) == 0;
            }
            public int GetHashCode() {
                let canonical = CanonicalString();
                return HashString(canonical);
            }
            public static UriHostNameType CheckHostName(string name) {
                if (name == null || name.Length == 0)
                {
                    return UriHostNameType.Unknown;
                }
                let span = name.AsUtf8Span();
                if (HasNonAscii (span))
                {
                    if (! IsValidUnicodeHost (span))
                    {
                        return UriHostNameType.Unknown;
                    }
                    if (! UriIdn.TryGetAsciiHost (span, out var asciiHost, out var hostStatus)) {
                        return UriHostNameType.Unknown;
                    }
                    return UriHostNameType.Dns;
                }
                if (IsValidIPv6 (span))
                {
                    return UriHostNameType.IPv6;
                }
                if (IsValidIPv4 (span))
                {
                    return UriHostNameType.IPv4;
                }
                if (IsValidDns (span))
                {
                    return UriHostNameType.Dns;
                }
                if (IsValidBasicHost (span))
                {
                    return UriHostNameType.Basic;
                }
                return UriHostNameType.Unknown;
            }
            public static bool CheckSchemeName(string schemeName) {
                if (schemeName == null || schemeName.Length == 0)
                {
                    return false;
                }
                let span = schemeName.AsUtf8Span();
                if (! IsAlpha (span[0]))
                {
                    return false;
                }
                var index = 1usize;
                while (index <span.Length)
                {
                    let current = span[index];
                    if (! IsSchemeChar (current))
                    {
                        return false;
                    }
                    index += 1;
                }
                return true;
            }
            public static int Compare(Uri uri1, Uri uri2, UriComponents partsToCompare, UriFormat compareFormat, StringComparison comparisonType) {
                if (uri1 == null && uri2 == null)
                {
                    return 0;
                }
                if (uri1 == null)
                {
                    return - 1;
                }
                if (uri2 == null)
                {
                    return 1;
                }
                let left = uri1.GetComponents(partsToCompare, compareFormat);
                let right = uri2.GetComponents(partsToCompare, compareFormat);
                return CompareStrings(left, right, comparisonType);
            }
            public static bool IsWellFormedUriString(string uriString, UriKind uriKind) {
                return TryCreate(uriString, uriKind, out var parsed);
            }
            public static string EscapeDataString(string s) {
                if (s == null)
                {
                    throw new ArgumentNullException("s");
                }
                return UriEscape.EscapeDataString(s);
            }
            public static string EscapeUriString(string s) {
                if (s == null)
                {
                    throw new ArgumentNullException("s");
                }
                return UriEscape.EscapeUriString(s);
            }
            public static string UnescapeDataString(string s) {
                if (s == null)
                {
                    throw new ArgumentNullException("s");
                }
                return UriEscape.UnescapeString(s, false);
            }
            public static int FromHex(char digit) {
                return UriEscape.FromHex(digit);
            }
            public static string HexEscape(char character) {
                return UriEscape.HexEscape(character);
            }
            public static char HexUnescape(string pattern, ref int index) {
                return UriEscape.HexUnescape(pattern, ref index);
            }
            public static bool IsHexDigit(char c) {
                return UriEscape.IsHexDigit(c);
            }
            public static bool IsHexEncoding(string pattern, int index) {
                return UriEscape.IsHexEncoding(pattern, index);
            }
            private void EnsureAbsolute() {
                if (! _isAbsolute)
                {
                    throw new InvalidOperationException("This operation is not supported for a relative URI");
                }
            }
            internal void ApplyParsed(string original, UriParts parsed) {
                _originalString = original;
                _isAbsolute = parsed.IsAbsolute;
                _hasAuthority = parsed.HasAuthority;
                _scheme = parsed.Scheme;
                _userInfo = parsed.UserInfo;
                _host = parsed.Host;
                _idnHost = parsed.IdnHost;
                _hostNameType = parsed.HostNameType;
                _port = parsed.Port;
                _portSpecified = parsed.PortSpecified;
                _path = parsed.Path;
                _query = parsed.Query;
                _fragment = parsed.Fragment;
                _userEscaped = parsed.UserEscaped;
                _isFile = parsed.IsFile;
                _isUnc = parsed.IsUnc;
                _isLoopback = parsed.IsLoopback;
                _cachedAbsoluteUri = null;
                _cachedToString = null;
                _cachedAbsolutePath = null;
                _cachedPathAndQuery = null;
                _segments = null;
                _segmentsComputed = false;
            }
            internal UriParts ToParts() {
                var parts = CoreIntrinsics.DefaultValue <UriParts >();
                parts.IsAbsolute = _isAbsolute;
                parts.HasAuthority = _hasAuthority;
                parts.Scheme = _scheme;
                parts.UserInfo = _userInfo;
                parts.Host = _host;
                parts.IdnHost = _idnHost;
                parts.HostNameType = _hostNameType;
                parts.Port = _port;
                parts.PortSpecified = _portSpecified;
                parts.Path = _path;
                parts.Query = _query;
                parts.Fragment = _fragment;
                parts.UserEscaped = _userEscaped;
                parts.IsFile = _isFile;
                parts.IsUnc = _isUnc;
                parts.IsLoopback = _isLoopback;
                return parts;
            }
            internal static Uri Resolve(Uri baseUri, UriParts relative) {
                var resultParts = CoreIntrinsics.DefaultValue <UriParts >();
                resultParts.IsAbsolute = true;
                resultParts.Scheme = baseUri._scheme;
                resultParts.IsFile = baseUri._isFile;
                resultParts.UserInfo = relative.UserInfo;
                resultParts.UserEscaped = relative.UserEscaped || baseUri._userEscaped;
                if (relative.HasAuthority)
                {
                    resultParts.HasAuthority = true;
                    resultParts.UserInfo = relative.UserInfo;
                    resultParts.Host = relative.Host;
                    resultParts.IdnHost = relative.IdnHost;
                    resultParts.HostNameType = relative.HostNameType;
                    resultParts.Port = relative.Port;
                    resultParts.PortSpecified = relative.PortSpecified;
                    resultParts.Path = RemoveDotSegments(relative.Path);
                    resultParts.Query = relative.Query;
                }
                else
                {
                    resultParts.HasAuthority = baseUri._hasAuthority;
                    resultParts.UserInfo = baseUri._userInfo;
                    resultParts.Host = baseUri._host;
                    resultParts.IdnHost = baseUri._idnHost;
                    resultParts.HostNameType = baseUri._hostNameType;
                    resultParts.Port = baseUri._port;
                    resultParts.PortSpecified = baseUri._portSpecified;
                    if (relative.Path == null || relative.Path.Length == 0)
                    {
                        resultParts.Path = baseUri._path;
                        resultParts.Query = (relative.Query == null || relative.Query.Length == 0) ?baseUri._query : relative.Query;
                    }
                    else
                    {
                        if (StartsWithSlash (relative.Path))
                        {
                            resultParts.Path = RemoveDotSegments(relative.Path);
                        }
                        else
                        {
                            let merged = MergePaths(baseUri._path, relative.Path, baseUri._hasAuthority);
                            resultParts.Path = RemoveDotSegments(merged);
                        }
                        resultParts.Query = relative.Query;
                    }
                }
                resultParts.Fragment = relative.Fragment;
                resultParts.IsUnc = resultParts.IsFile && resultParts.Host != null && resultParts.Host.Length >0;
                resultParts.IsLoopback = IsLoopbackHost(resultParts.Host, resultParts.HostNameType, resultParts.IsFile);
                if (! resultParts.HasAuthority && resultParts.IsFile)
                {
                    resultParts.HasAuthority = true;
                }
                let text = BuildOriginalString(resultParts);
                return new Uri(text, UriKind.Absolute);
            }
            internal static string BuildOriginalString(UriParts parts) {
                var buffer = FVec.WithCapacity <byte >(64);
                AppendString(ref buffer, parts.Scheme);
                AppendString(ref buffer, ":");
                if (parts.HasAuthority)
                {
                    AppendString(ref buffer, "//");
                    if (parts.UserInfo != null && parts.UserInfo.Length >0)
                    {
                        AppendString(ref buffer, UriEscape.EscapeComponent(parts.UserInfo, UriEscapeComponent.UserInfo, true));
                        AppendString(ref buffer, "@");
                    }
                    AppendString(ref buffer, BuildHostForAuthority(parts.Host, parts.IdnHost, parts.HostNameType));
                    let portValue = parts.PortSpecified ?parts.Port : - 1;
                    if (portValue >= 0 && ! IsDefaultPort (parts.Scheme, portValue))
                    {
                        AppendString(ref buffer, ":");
                        AppendString(ref buffer, portValue.ToString());
                    }
                }
                let path = parts.Path;
                if (path == null || path.Length == 0)
                {
                    if (parts.HasAuthority)
                    {
                        AppendString(ref buffer, "/");
                    }
                }
                else
                {
                    AppendString(ref buffer, UriEscape.EscapeComponent(path, UriEscapeComponent.Path, true));
                }
                if (parts.Query != null && parts.Query.Length >0)
                {
                    AppendString(ref buffer, "?");
                    AppendString(ref buffer, UriEscape.EscapeComponent(parts.Query, UriEscapeComponent.Query, true));
                }
                if (parts.Fragment != null && parts.Fragment.Length >0)
                {
                    AppendString(ref buffer, "#");
                    AppendString(ref buffer, UriEscape.EscapeComponent(parts.Fragment, UriEscapeComponent.Fragment, true));
                }
                let result = Utf8String.FromSpan(FVec.AsReadOnlySpan <byte >(in buffer));
                FVecIntrinsics.chic_rt_vec_drop(ref buffer);
                return result;
            }
            internal string BuildUriString(UriFormat format) {
                var buffer = FVec.WithCapacity <byte >(64);
                if (_isAbsolute)
                {
                    AppendString(ref buffer, _scheme);
                    AppendString(ref buffer, ":");
                    if (_hasAuthority)
                    {
                        AppendString(ref buffer, "//");
                        if (_userInfo != null && _userInfo.Length >0)
                        {
                            AppendString(ref buffer, FormatComponent(_userInfo, format, UriEscapeComponent.UserInfo));
                            AppendString(ref buffer, "@");
                        }
                        AppendString(ref buffer, BuildHostForAuthority(_host, _idnHost, _hostNameType));
                        let portValue = _portSpecified ?_port : - 1;
                        if (portValue >= 0 && ! IsDefaultPort (_scheme, portValue))
                        {
                            AppendString(ref buffer, ":");
                            AppendString(ref buffer, portValue.ToString());
                        }
                    }
                }
                let path = BuildPath(format);
                if (path.Length >0)
                {
                    AppendString(ref buffer, path);
                }
                else if (_isAbsolute && _hasAuthority)
                {
                    AppendString(ref buffer, "/");
                }
                if (_query != null && _query.Length >0)
                {
                    AppendString(ref buffer, "?");
                    AppendString(ref buffer, FormatComponent(_query, format, UriEscapeComponent.Query));
                }
                if (_fragment != null && _fragment.Length >0)
                {
                    AppendString(ref buffer, "#");
                    AppendString(ref buffer, FormatComponent(_fragment, format, UriEscapeComponent.Fragment));
                }
                let result = Utf8String.FromSpan(FVec.AsReadOnlySpan <byte >(in buffer));
                FVecIntrinsics.chic_rt_vec_drop(ref buffer);
                return result;
            }
            internal string BuildAuthority(bool includeUserInfo, bool includePort, UriFormat format) {
                var buffer = FVec.WithCapacity <byte >(32);
                if (includeUserInfo && _userInfo != null && _userInfo.Length >0)
                {
                    AppendString(ref buffer, FormatComponent(_userInfo, format, UriEscapeComponent.UserInfo));
                    AppendString(ref buffer, "@");
                }
                AppendString(ref buffer, BuildHostForAuthority(_host, _idnHost, _hostNameType));
                if (includePort)
                {
                    let portValue = _portSpecified ?_port : - 1;
                    if (portValue >= 0 && ! IsDefaultPort (_scheme, portValue))
                    {
                        AppendString(ref buffer, ":");
                        AppendString(ref buffer, portValue.ToString());
                    }
                }
                let result = Utf8String.FromSpan(FVec.AsReadOnlySpan <byte >(in buffer));
                FVecIntrinsics.chic_rt_vec_drop(ref buffer);
                return result;
            }
            internal string BuildPath(UriFormat format) {
                if (_path == null || _path.Length == 0)
                {
                    return Std.Runtime.StringRuntime.Create();
                }
                return FormatComponent(_path, format, UriEscapeComponent.Path);
            }
            internal string BuildPathAndQuery(UriFormat format) {
                var buffer = FVec.WithCapacity <byte >(64);
                let path = BuildPath(format);
                if (path.Length >0)
                {
                    AppendString(ref buffer, path);
                }
                if (_query != null && _query.Length >0)
                {
                    AppendString(ref buffer, "?");
                    AppendString(ref buffer, FormatComponent(_query, format, UriEscapeComponent.Query));
                }
                let result = Utf8String.FromSpan(FVec.AsReadOnlySpan <byte >(in buffer));
                FVecIntrinsics.chic_rt_vec_drop(ref buffer);
                return result;
            }
            private string BuildLeftPartAuthority() {
                var buffer = FVec.WithCapacity <byte >(64);
                AppendString(ref buffer, _scheme);
                AppendString(ref buffer, "://");
                AppendString(ref buffer, BuildAuthority(true, true, UriFormat.UriEscaped));
                let result = Utf8String.FromSpan(FVec.AsReadOnlySpan <byte >(in buffer));
                FVecIntrinsics.chic_rt_vec_drop(ref buffer);
                return result;
            }
            private string BuildLeftPartPath() {
                var buffer = FVec.WithCapacity <byte >(64);
                AppendString(ref buffer, BuildLeftPartAuthority());
                AppendString(ref buffer, AbsolutePath);
                let result = Utf8String.FromSpan(FVec.AsReadOnlySpan <byte >(in buffer));
                FVecIntrinsics.chic_rt_vec_drop(ref buffer);
                return result;
            }
            private string BuildLeftPartQuery() {
                var buffer = FVec.WithCapacity <byte >(64);
                AppendString(ref buffer, BuildLeftPartPath());
                AppendString(ref buffer, Query);
                let result = Utf8String.FromSpan(FVec.AsReadOnlySpan <byte >(in buffer));
                FVecIntrinsics.chic_rt_vec_drop(ref buffer);
                return result;
            }
            private string[] BuildSegments() {
                let path = AbsolutePath;
                if (path == null || path.Length == 0)
                {
                    let emptySegments = 0;
                    return new string[emptySegments];
                }
                let span = path.AsUtf8Span();
                var hasLeadingSlash = span.Length >0 && span[0] == NumericUnchecked.ToByte('/');
                var count = 0;
                if (hasLeadingSlash)
                {
                    count += 1;
                }
                var index = hasLeadingSlash ?1usize : 0usize;
                while (index <span.Length)
                {
                    var next = index;
                    while (next <span.Length && span[next] != NumericUnchecked.ToByte ('/'))
                    {
                        next += 1;
                    }
                    var length = next - index;
                    if (next <span.Length)
                    {
                        length += 1;
                        if (length == 1 && next + 1 == span.Length)
                        {
                            break;
                        }
                    }
                    count += 1;
                    index = next + 1;
                }
                var segments = new string[count];
                var segIndex = 0;
                if (hasLeadingSlash)
                {
                    segments[segIndex] = "/";
                    segIndex += 1;
                }
                index = hasLeadingSlash ?1usize : 0usize;
                while (index <span.Length && segIndex <count)
                {
                    var next = index;
                    while (next <span.Length && span[next] != NumericUnchecked.ToByte ('/'))
                    {
                        next += 1;
                    }
                    var length = next - index;
                    if (next <span.Length)
                    {
                        length += 1;
                        if (length == 1 && next + 1 == span.Length)
                        {
                            break;
                        }
                    }
                    let slice = span.Slice(index, length);
                    let segment = Utf8String.FromSpan(slice);
                    segments[segIndex] = segment;
                    segIndex += 1;
                    index = next + 1;
                }
                return segments;
            }
            private string CanonicalString() {
                if (_isAbsolute)
                {
                    return AbsoluteUri;
                }
                return ToString();
            }
            internal static int HashString(string value) {
                if (value == null)
                {
                    return 0;
                }
                let span = value.AsUtf8Span();
                var hash = 2166136261u;
                var index = 0usize;
                while (index <span.Length)
                {
                    hash = (hash ^ span[index]) * 16777619u;
                    index += 1;
                }
                return(int) hash;
            }
            internal static string FormatComponent(string value, UriFormat format, UriEscapeComponent component) {
                if (value == null || value.Length == 0)
                {
                    return Std.Runtime.StringRuntime.Create();
                }
                switch (format)
                {
                    case UriFormat.Unescaped:
                        return UriEscape.UnescapeString(value, false);
                    case UriFormat.SafeUnescaped:
                        return UriEscape.UnescapeString(value, true);
                    default :
                        return UriEscape.EscapeComponent(value, component, true);
                    }
                }
                internal static void AppendString(ref VecPtr buffer, string value) {
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
                internal static string Concat2(string left, string right) {
                    var buffer = FVec.WithCapacity <byte >(NumericUnchecked.ToUSize((left == null ?0 : left.Length) + (right == null ?0 : right.Length)));
                    AppendString(ref buffer, left);
                    AppendString(ref buffer, right);
                    let result = Utf8String.FromSpan(FVec.AsReadOnlySpan <byte >(in buffer));
                    FVecIntrinsics.chic_rt_vec_drop(ref buffer);
                    return result;
                }
                internal static string Concat3(string first, string second, string third) {
                    var buffer = FVec.WithCapacity <byte >(NumericUnchecked.ToUSize((first == null ?0 : first.Length) + (second == null ?0 : second.Length) + (third == null ?0 : third.Length)));
                    AppendString(ref buffer, first);
                    AppendString(ref buffer, second);
                    AppendString(ref buffer, third);
                    let result = Utf8String.FromSpan(FVec.AsReadOnlySpan <byte >(in buffer));
                    FVecIntrinsics.chic_rt_vec_drop(ref buffer);
                    return result;
                }
                internal static string BuildHostForAuthority(string host, string idnHost, UriHostNameType hostType) {
                    if (hostType == UriHostNameType.IPv6)
                    {
                        return Concat3("[", host, "]");
                    }
                    if (idnHost != null && idnHost.Length >0)
                    {
                        return idnHost;
                    }
                    if (host == null)
                    {
                        return Std.Runtime.StringRuntime.Create();
                    }
                    return host;
                }
                internal static bool TryParseCore(string uriString, UriKind uriKind, out UriParts parts, out string error) {
                    parts = CoreIntrinsics.DefaultValue <UriParts >();
                    error = Std.Runtime.StringRuntime.Create();
                    let span = uriString.AsUtf8Span();
                    if (span.Length == 0)
                    {
                        if (uriKind == UriKind.Absolute)
                        {
                            error = "Absolute URI must not be empty";
                            return false;
                        }
                        parts.IsAbsolute = false;
                        parts.Path = Std.Runtime.StringRuntime.Create();
                        parts.Scheme = Std.Runtime.StringRuntime.Create();
                        parts.UserInfo = Std.Runtime.StringRuntime.Create();
                        parts.Host = Std.Runtime.StringRuntime.Create();
                        parts.Query = Std.Runtime.StringRuntime.Create();
                        parts.Fragment = Std.Runtime.StringRuntime.Create();
                        return true;
                    }
                    var schemeEnd = FindScheme(span);
                    var hasScheme = schemeEnd >= 0;
                    if (hasScheme && ! IsValidScheme (span, schemeEnd))
                    {
                        if (uriKind == UriKind.Absolute)
                        {
                            error = "Invalid scheme";
                            return false;
                        }
                        hasScheme = false;
                    }
                    if (hasScheme)
                    {
                        if (uriKind == UriKind.Relative)
                        {
                            error = "Relative URI required";
                            return false;
                        }
                    }
                    else
                    {
                        if (uriKind == UriKind.Absolute)
                        {
                            error = "Absolute URI required";
                            return false;
                        }
                    }
                    var index = 0usize;
                    parts.Scheme = Std.Runtime.StringRuntime.Create();
                    if (hasScheme)
                    {
                        parts.IsAbsolute = true;
                        parts.Scheme = ToLowerAscii(span.Slice(0, NumericUnchecked.ToUSize(schemeEnd)));
                        index = NumericUnchecked.ToUSize(schemeEnd + 1);
                    }
                    else
                    {
                        parts.IsAbsolute = false;
                    }
                    parts.HasAuthority = false;
                    parts.UserInfo = Std.Runtime.StringRuntime.Create();
                    parts.Host = Std.Runtime.StringRuntime.Create();
                    parts.IdnHost = Std.Runtime.StringRuntime.Create();
                    parts.HostNameType = UriHostNameType.Unknown;
                    parts.Port = - 1;
                    parts.PortSpecified = false;
                    parts.Path = Std.Runtime.StringRuntime.Create();
                    parts.Query = Std.Runtime.StringRuntime.Create();
                    parts.Fragment = Std.Runtime.StringRuntime.Create();
                    parts.UserEscaped = false;
                    parts.IsFile = hasScheme && EqualsAscii(parts.Scheme, UriSchemeFile);
                    parts.IsUnc = false;
                    parts.IsLoopback = false;
                    if (hasScheme)
                    {
                        if (index + 1 <span.Length && span[index] == NumericUnchecked.ToByte ('/') && span[index + 1] == NumericUnchecked.ToByte ('/'))
                        {
                            parts.HasAuthority = true;
                            index += 2;
                        }
                        else if (parts.IsFile)
                        {
                            if (index <span.Length && span[index] == NumericUnchecked.ToByte ('/'))
                            {
                                parts.HasAuthority = true;
                            }
                        }
                    }
                    else if (index + 1 <span.Length && span[0] == NumericUnchecked.ToByte ('/') && span[1] == NumericUnchecked.ToByte ('/'))
                    {
                        parts.HasAuthority = true;
                        index = 2;
                    }
                    if (parts.HasAuthority)
                    {
                        let authorityStart = index;
                        var authorityEnd = authorityStart;
                        while (authorityEnd <span.Length)
                        {
                            let current = span[authorityEnd];
                            if (current == NumericUnchecked.ToByte ('/') || current == NumericUnchecked.ToByte ('?') || current == NumericUnchecked.ToByte ('#'))
                            {
                                break;
                            }
                            authorityEnd += 1;
                        }
                        let authoritySpan = span.Slice(authorityStart, authorityEnd - authorityStart);
                        index = authorityEnd;
                        if (! ParseAuthority (authoritySpan, parts.IsFile, ref parts, out error)) {
                            return false;
                        }
                    }
                    let pathStart = index;
                    var pathEnd = pathStart;
                    while (pathEnd <span.Length)
                    {
                        let current = span[pathEnd];
                        if (current == NumericUnchecked.ToByte ('?') || current == NumericUnchecked.ToByte ('#'))
                        {
                            break;
                        }
                        pathEnd += 1;
                    }
                    let pathSpan = span.Slice(pathStart, pathEnd - pathStart);
                    if (pathSpan.Length >0)
                    {
                        if (! ValidateComponent (pathSpan, true, true, out var pathEscaped)) {
                            error = "Invalid path";
                            return false;
                        }
                        parts.UserEscaped = parts.UserEscaped || pathEscaped;
                        parts.Path = Utf8String.FromSpan(pathSpan);
                    }
                    else
                    {
                        parts.Path = Std.Runtime.StringRuntime.Create();
                    }
                    index = pathEnd;
                    if (index <span.Length && span[index] == NumericUnchecked.ToByte ('?'))
                    {
                        let queryStart = index + 1;
                        var queryEnd = queryStart;
                        while (queryEnd <span.Length && span[queryEnd] != NumericUnchecked.ToByte ('#'))
                        {
                            queryEnd += 1;
                        }
                        let querySpan = span.Slice(queryStart, queryEnd - queryStart);
                        if (! ValidateComponent (querySpan, true, true, out var queryEscaped)) {
                            error = "Invalid query";
                            return false;
                        }
                        parts.UserEscaped = parts.UserEscaped || queryEscaped;
                        parts.Query = Utf8String.FromSpan(querySpan);
                        index = queryEnd;
                    }
                    if (index <span.Length && span[index] == NumericUnchecked.ToByte ('#'))
                    {
                        let fragStart = index + 1;
                        let fragSpan = span.Slice(fragStart, span.Length - fragStart);
                        if (! ValidateComponent (fragSpan, true, true, out var fragEscaped)) {
                            error = "Invalid fragment";
                            return false;
                        }
                        parts.UserEscaped = parts.UserEscaped || fragEscaped;
                        parts.Fragment = Utf8String.FromSpan(fragSpan);
                    }
                    if (parts.IsAbsolute)
                    {
                        if (parts.Path != null && parts.Path.Length >0)
                        {
                            parts.Path = RemoveDotSegments(parts.Path);
                        }
                        else if (parts.HasAuthority)
                        {
                            parts.Path = "/";
                        }
                    }
                    parts.IsUnc = parts.IsFile && parts.Host != null && parts.Host.Length >0;
                    parts.IsLoopback = IsLoopbackHost(parts.Host, parts.HostNameType, parts.IsFile);
                    if (parts.IsFile && parts.HasAuthority == false)
                    {
                        parts.HasAuthority = true;
                    }
                    return true;
                }
                internal static int FindScheme(ReadOnlySpan <byte >span) {
                    var index = 0;
                    while (index <span.Length)
                    {
                        let current = span[index];
                        if (current == NumericUnchecked.ToByte (':'))
                        {
                            return NumericUnchecked.ToInt32(index);
                        }
                        if (current == NumericUnchecked.ToByte ('/') || current == NumericUnchecked.ToByte ('?') || current == NumericUnchecked.ToByte ('#'))
                        {
                            break;
                        }
                        index += 1;
                    }
                    return - 1;
                }
                internal static bool IsValidScheme(ReadOnlySpan <byte >span, int schemeEnd) {
                    if (schemeEnd <= 0)
                    {
                        return false;
                    }
                    if (! IsAlpha (span[0]))
                    {
                        return false;
                    }
                    var index = 1usize;
                    while (index <NumericUnchecked.ToUSize (schemeEnd))
                    {
                        if (! IsSchemeChar (span[index]))
                        {
                            return false;
                        }
                        index += 1;
                    }
                    return true;
                }
                internal static bool ParseAuthority(ReadOnlySpan <byte >authority, bool isFile, ref UriParts parts, out string error) {
                    error = Std.Runtime.StringRuntime.Create();
                    var userEscaped = false;
                    var userInfo = Std.Runtime.StringRuntime.Create();
                    var hostSpan = authority;
                    var atIndex = - 1;
                    var index = 0usize;
                    while (index <authority.Length)
                    {
                        if (authority[index] == NumericUnchecked.ToByte ('@'))
                        {
                            atIndex = NumericUnchecked.ToInt32(index);
                        }
                        index += 1;
                    }
                    if (atIndex >= 0)
                    {
                        let userSpan = authority.Slice(0, NumericUnchecked.ToUSize(atIndex));
                        if (! ValidateUserInfo (userSpan, out var escaped)) {
                            error = "Invalid user info";
                            return false;
                        }
                        userEscaped = escaped;
                        userInfo = Utf8String.FromSpan(userSpan);
                        hostSpan = authority.Slice(NumericUnchecked.ToUSize(atIndex + 1), authority.Length - NumericUnchecked.ToUSize(atIndex + 1));
                    }
                    var host = Std.Runtime.StringRuntime.Create();
                    var hostType = UriHostNameType.Unknown;
                    var port = - 1;
                    var portSpecified = false;
                    if (hostSpan.Length == 0)
                    {
                        if (! isFile)
                        {
                            error = "Invalid host";
                            return false;
                        }
                    }
                    else if (hostSpan[0] == NumericUnchecked.ToByte ('['))
                    {
                        let closing = FindChar(hostSpan, NumericUnchecked.ToByte(']'));
                        if (closing <0)
                        {
                            error = "Invalid IPv6 host";
                            return false;
                        }
                        let hostValue = hostSpan.Slice(1, NumericUnchecked.ToUSize(closing) - 1);
                        if (! IsValidIPv6 (hostValue))
                        {
                            error = "Invalid IPv6 host";
                            return false;
                        }
                        host = ToLowerAscii(hostValue);
                        parts.IdnHost = ToLowerAscii(hostValue);
                        hostType = UriHostNameType.IPv6;
                        if (NumericUnchecked.ToUSize (closing + 1) <hostSpan.Length)
                        {
                            if (hostSpan[NumericUnchecked.ToUSize (closing + 1)] != NumericUnchecked.ToByte (':'))
                            {
                                error = "Invalid host";
                                return false;
                            }
                            let portSpan = hostSpan.Slice(NumericUnchecked.ToUSize(closing + 2), hostSpan.Length - NumericUnchecked.ToUSize(closing + 2));
                            if (! ParsePort (portSpan, out port)) {
                                error = "Invalid port";
                                return false;
                            }
                            portSpecified = true;
                        }
                    }
                    else
                    {
                        var colonIndex = - 1;
                        var colonCount = 0;
                        index = 0usize;
                        while (index <hostSpan.Length)
                        {
                            if (hostSpan[index] == NumericUnchecked.ToByte (':'))
                            {
                                colonCount += 1;
                                colonIndex = NumericUnchecked.ToInt32(index);
                            }
                            index += 1;
                        }
                        if (colonCount >1)
                        {
                            error = "Invalid host";
                            return false;
                        }
                        var hostValueSpan = hostSpan;
                        if (colonIndex >= 0)
                        {
                            hostValueSpan = hostSpan.Slice(0, NumericUnchecked.ToUSize(colonIndex));
                            let portSpan = hostSpan.Slice(NumericUnchecked.ToUSize(colonIndex + 1), hostSpan.Length - NumericUnchecked.ToUSize(colonIndex + 1));
                            if (! ParsePort (portSpan, out port)) {
                                error = "Invalid port";
                                return false;
                            }
                            portSpecified = true;
                        }
                        if (! ValidateHost (hostValueSpan, out hostType, out var hostValue, out var idnValue)) {
                            error = "Invalid host";
                            return false;
                        }
                        host = hostValue;
                        parts.IdnHost = idnValue;
                    }
                    parts.UserInfo = userInfo;
                    parts.UserEscaped = userEscaped;
                    parts.Host = host;
                    parts.HostNameType = hostType;
                    parts.Port = port;
                    parts.PortSpecified = portSpecified;
                    return true;
                }
                internal static bool ValidateHost(ReadOnlySpan <byte >hostSpan, out UriHostNameType hostType, out string host,
                out string idnHost) {
                    hostType = UriHostNameType.Unknown;
                    host = Std.Runtime.StringRuntime.Create();
                    idnHost = Std.Runtime.StringRuntime.Create();
                    if (hostSpan.Length == 0)
                    {
                        return true;
                    }
                    if (HasNonAscii (hostSpan))
                    {
                        if (! IsValidUnicodeHost (hostSpan))
                        {
                            return false;
                        }
                        if (! UriIdn.TryGetAsciiHost (hostSpan, out idnHost, out var hostStatus)) {
                            return false;
                        }
                        hostType = UriHostNameType.Dns;
                        host = Utf8String.FromSpan(hostSpan);
                        return true;
                    }
                    if (IsValidIPv6 (hostSpan))
                    {
                        hostType = UriHostNameType.IPv6;
                        host = ToLowerAscii(hostSpan);
                        idnHost = ToLowerAscii(hostSpan);
                        return true;
                    }
                    if (IsValidIPv4 (hostSpan))
                    {
                        hostType = UriHostNameType.IPv4;
                        host = ToLowerAscii(hostSpan);
                        idnHost = ToLowerAscii(hostSpan);
                        return true;
                    }
                    if (IsValidDns (hostSpan))
                    {
                        hostType = UriHostNameType.Dns;
                        host = NormalizeHostAscii(hostSpan);
                        idnHost = NormalizeHostAscii(hostSpan);
                        return true;
                    }
                    if (IsValidBasicHost (hostSpan))
                    {
                        hostType = UriHostNameType.Basic;
                        host = NormalizeHostAscii(hostSpan);
                        idnHost = NormalizeHostAscii(hostSpan);
                        return true;
                    }
                    return false;
                }
                internal static bool ValidateComponent(ReadOnlySpan <byte >component, bool allowNonAscii, bool allowSlash,
                out bool userEscaped) {
                    userEscaped = false;
                    var index = 0usize;
                    while (index <component.Length)
                    {
                        let current = component[index];
                        if (current == NumericUnchecked.ToByte ('%'))
                        {
                            if (index + 2 >= component.Length)
                            {
                                return false;
                            }
                            if (! UriEscape.IsHexDigit (component[index + 1]) || ! UriEscape.IsHexDigit (component[index + 2]))
                            {
                                return false;
                            }
                            userEscaped = true;
                            index += 3;
                            continue;
                        }
                        if (current == NumericUnchecked.ToByte (' ') || current <NumericUnchecked.ToByte (0x20) || current == NumericUnchecked.ToByte (0x7F))
                        {
                            return false;
                        }
                        if (! allowSlash && current == NumericUnchecked.ToByte ('/'))
                        {
                            return false;
                        }
                        if (! allowNonAscii && current >= NumericUnchecked.ToByte (0x80))
                        {
                            return false;
                        }
                        if (current == NumericUnchecked.ToByte ('\\'))
                        {
                            return false;
                        }
                        index += 1;
                    }
                    return true;
                }
                internal static bool ValidateUserInfo(ReadOnlySpan <byte >userInfo, out bool userEscaped) {
                    userEscaped = false;
                    var index = 0usize;
                    while (index <userInfo.Length)
                    {
                        let current = userInfo[index];
                        if (current == NumericUnchecked.ToByte ('%'))
                        {
                            if (index + 2 >= userInfo.Length)
                            {
                                return false;
                            }
                            if (! UriEscape.IsHexDigit (userInfo[index + 1]) || ! UriEscape.IsHexDigit (userInfo[index + 2]))
                            {
                                return false;
                            }
                            userEscaped = true;
                            index += 3;
                            continue;
                        }
                        if (current >= NumericUnchecked.ToByte (0x80))
                        {
                            return false;
                        }
                        if (! (IsUnreservedAscii (current) || IsSubDelimAscii (current) || current == NumericUnchecked.ToByte (':')))
                        {
                            return false;
                        }
                        index += 1;
                    }
                    return true;
                }
                internal static bool IsValidUnicodeHost(ReadOnlySpan <byte >span) {
                    var index = 0usize;
                    var labelLength = 0usize;
                    while (index <span.Length)
                    {
                        let current = span[index];
                        if (current == NumericUnchecked.ToByte ('.'))
                        {
                            if (labelLength == 0)
                            {
                                return false;
                            }
                            labelLength = 0;
                            index += 1;
                            continue;
                        }
                        if (current <NumericUnchecked.ToByte (0x20) || current == NumericUnchecked.ToByte (0x7F))
                        {
                            return false;
                        }
                        if (current == NumericUnchecked.ToByte (' ') || current == NumericUnchecked.ToByte ('/') || current == NumericUnchecked.ToByte ('\\') || current == NumericUnchecked.ToByte ('?') || current == NumericUnchecked.ToByte ('#') || current == NumericUnchecked.ToByte ('@') || current == NumericUnchecked.ToByte (':') || current == NumericUnchecked.ToByte ('[') || current == NumericUnchecked.ToByte (']') || current == NumericUnchecked.ToByte ('%'))
                        {
                            return false;
                        }
                        labelLength += 1;
                        index += 1;
                    }
                    if (labelLength == 0)
                    {
                        return false;
                    }
                    return true;
                }
                internal static string NormalizeHostAscii(ReadOnlySpan <byte >span) {
                    return Utf8String.FromSpan(span);
                }
                internal static byte ToUpperHex(byte value) {
                    if (value >= NumericUnchecked.ToByte ('a') && value <= NumericUnchecked.ToByte ('f'))
                    {
                        return NumericUnchecked.ToByte(NumericUnchecked.ToInt32(value) - 32);
                    }
                    return value;
                }
                internal static bool IsUnreservedAscii(byte value) {
                    return(value >= NumericUnchecked.ToByte('A') && value <= NumericUnchecked.ToByte('Z')) || (value >= NumericUnchecked.ToByte('a') && value <= NumericUnchecked.ToByte('z')) || (value >= NumericUnchecked.ToByte('0') && value <= NumericUnchecked.ToByte('9')) || value == NumericUnchecked.ToByte('-') || value == NumericUnchecked.ToByte('.') || value == NumericUnchecked.ToByte('_') || value == NumericUnchecked.ToByte('~');
                }
                internal static bool IsSubDelimAscii(byte value) {
                    return value == NumericUnchecked.ToByte('!') || value == NumericUnchecked.ToByte('$') || value == NumericUnchecked.ToByte('&') || value == NumericUnchecked.ToByte('(') || value == NumericUnchecked.ToByte(')') || value == NumericUnchecked.ToByte('*') || value == NumericUnchecked.ToByte('+') || value == NumericUnchecked.ToByte(',') || value == NumericUnchecked.ToByte(';') || value == NumericUnchecked.ToByte('=') || value == NumericUnchecked.ToByte('\'');
                }
                internal static bool ParsePort(ReadOnlySpan <byte >span, out int port) {
                    port = - 1;
                    if (span.Length == 0)
                    {
                        return false;
                    }
                    if (! Std.Numeric.NumericParse.TryParseInt32 (span, out port)) {
                        return false;
                    }
                    if (port <0 || port >65535)
                    {
                        return false;
                    }
                    return true;
                }
                internal static string RemoveDotSegments(string path) {
                    if (path == null)
                    {
                        return Std.Runtime.StringRuntime.Create();
                    }
                    return path;
                }
                internal static void RemoveLastSegment(ref VecPtr output) {
                    let span = FVec.AsReadOnlySpan <byte >(in output);
                    var index = span.Length;
                    while (index >0)
                    {
                        index -= 1;
                        if (span[index] == NumericUnchecked.ToByte ('/'))
                        {
                            FVec.Truncate(ref output, index);
                            return;
                        }
                    }
                    FVec.Truncate(ref output, 0);
                }
                internal static string MergePaths(string basePath, string relativePath, bool baseHasAuthority) {
                    if (basePath == null || basePath.Length == 0)
                    {
                        if (baseHasAuthority)
                        {
                            return Concat2("/", relativePath);
                        }
                        return relativePath;
                    }
                    let baseSpan = basePath.AsUtf8Span();
                    var lastSlash = - 1;
                    var index = 0usize;
                    while (index <baseSpan.Length)
                    {
                        if (baseSpan[index] == NumericUnchecked.ToByte ('/'))
                        {
                            lastSlash = NumericUnchecked.ToInt32(index);
                        }
                        index += 1;
                    }
                    if (lastSlash <0)
                    {
                        return relativePath;
                    }
                    let prefix = baseSpan.Slice(0, NumericUnchecked.ToUSize(lastSlash + 1));
                    let prefixText = Utf8String.FromSpan(prefix);
                    return Concat2(prefixText, relativePath);
                }
                internal static bool StartsWithSlash(string value) {
                    if (value == null || value.Length == 0)
                    {
                        return false;
                    }
                    return value[0] == '/';
                }
                internal static bool StartsWith(ReadOnlySpan <byte >span, usize start, str literal) {
                    let literalSpan = literal.AsSpan();
                    if (literalSpan.Length == 0)
                    {
                        return true;
                    }
                    if (start + literalSpan.Length >span.Length)
                    {
                        return false;
                    }
                    var index = 0usize;
                    while (index <literalSpan.Length)
                    {
                        if (span[start + index] != NumericUnchecked.ToByte (literalSpan[index]))
                        {
                            return false;
                        }
                        index += 1;
                    }
                    return true;
                }
                internal static int FindChar(ReadOnlySpan <byte >span, byte value) {
                    var index = 0usize;
                    while (index <span.Length)
                    {
                        if (span[index] == value)
                        {
                            return NumericUnchecked.ToInt32(index);
                        }
                        index += 1;
                    }
                    return - 1;
                }
                internal static string ToLowerAscii(ReadOnlySpan <byte >span) {
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
                    let result = Utf8String.FromSpan(FVec.AsReadOnlySpan <byte >(in buffer));
                    FVecIntrinsics.chic_rt_vec_drop(ref buffer);
                    return result;
                }
                internal static bool HasNonAscii(ReadOnlySpan <byte >span) {
                    var index = 0usize;
                    while (index <span.Length)
                    {
                        if (span[index] >= NumericUnchecked.ToByte (0x80))
                        {
                            return true;
                        }
                        index += 1;
                    }
                    return false;
                }
                internal static bool IsValidIPv4(ReadOnlySpan <byte >span) {
                    var segments = 0;
                    var index = 0usize;
                    while (index <span.Length)
                    {
                        if (segments >3)
                        {
                            return false;
                        }
                        var value = 0;
                        var digitCount = 0;
                        while (index <span.Length && span[index] != NumericUnchecked.ToByte ('.'))
                        {
                            let current = span[index];
                            if (current <NumericUnchecked.ToByte ('0') || current >NumericUnchecked.ToByte ('9'))
                            {
                                return false;
                            }
                            value = value * 10 + NumericUnchecked.ToInt32(current - NumericUnchecked.ToByte('0'));
                            if (value >255)
                            {
                                return false;
                            }
                            digitCount += 1;
                            index += 1;
                        }
                        if (digitCount == 0)
                        {
                            return false;
                        }
                        segments += 1;
                        if (index <span.Length && span[index] == NumericUnchecked.ToByte ('.'))
                        {
                            index += 1;
                        }
                    }
                    return segments == 4;
                }
                internal static bool IsValidIPv6(ReadOnlySpan <byte >span) {
                    if (span.Length == 0)
                    {
                        return false;
                    }
                    var segments = 0;
                    var index = 0usize;
                    var doubleColon = false;
                    while (index <span.Length)
                    {
                        if (span[index] == NumericUnchecked.ToByte (':'))
                        {
                            if (index + 1 <span.Length && span[index + 1] == NumericUnchecked.ToByte (':'))
                            {
                                if (doubleColon)
                                {
                                    return false;
                                }
                                doubleColon = true;
                                index += 2;
                                if (index >= span.Length)
                                {
                                    return true;
                                }
                                continue;
                            }
                            return false;
                        }
                        var digitCount = 0;
                        while (index <span.Length && span[index] != NumericUnchecked.ToByte (':'))
                        {
                            if (! UriEscape.IsHexDigit (span[index]))
                            {
                                return false;
                            }
                            digitCount += 1;
                            if (digitCount >4)
                            {
                                return false;
                            }
                            index += 1;
                        }
                        segments += 1;
                        if (index <span.Length && span[index] == NumericUnchecked.ToByte (':'))
                        {
                            index += 1;
                        }
                    }
                    if (segments >8)
                    {
                        return false;
                    }
                    if (! doubleColon && segments != 8)
                    {
                        return false;
                    }
                    return true;
                }
                internal static bool IsValidDns(ReadOnlySpan <byte >span) {
                    var index = 0usize;
                    var labelLength = 0usize;
                    var totalLength = 0usize;
                    var prevHyphen = false;
                    while (index <span.Length)
                    {
                        let current = span[index];
                        if (current == NumericUnchecked.ToByte ('.'))
                        {
                            if (labelLength == 0 || labelLength >63 || prevHyphen)
                            {
                                return false;
                            }
                            labelLength = 0;
                            prevHyphen = false;
                            index += 1;
                            totalLength += 1;
                            continue;
                        }
                        if (! (IsAlpha (current) || IsDigit (current) || current == NumericUnchecked.ToByte ('-')))
                        {
                            return false;
                        }
                        if (labelLength == 0 && current == NumericUnchecked.ToByte ('-'))
                        {
                            return false;
                        }
                        labelLength += 1;
                        totalLength += 1;
                        if (labelLength >63)
                        {
                            return false;
                        }
                        prevHyphen = current == NumericUnchecked.ToByte('-');
                        index += 1;
                    }
                    if (labelLength == 0 || labelLength >63 || prevHyphen)
                    {
                        return false;
                    }
                    return totalLength <= 255;
                }
                internal static bool IsValidBasicHost(ReadOnlySpan <byte >span) {
                    var index = 0usize;
                    var labelLength = 0usize;
                    while (index <span.Length)
                    {
                        let current = span[index];
                        if (current == NumericUnchecked.ToByte ('%'))
                        {
                            if (index + 2 >= span.Length)
                            {
                                return false;
                            }
                            if (! UriEscape.IsHexDigit (span[index + 1]) || ! UriEscape.IsHexDigit (span[index + 2]))
                            {
                                return false;
                            }
                            labelLength += 1;
                            index += 3;
                            continue;
                        }
                        if (current == NumericUnchecked.ToByte ('.'))
                        {
                            if (labelLength == 0)
                            {
                                return false;
                            }
                            labelLength = 0;
                            index += 1;
                            continue;
                        }
                        if (! (IsUnreservedAscii (current) || IsSubDelimAscii (current)))
                        {
                            return false;
                        }
                        labelLength += 1;
                        index += 1;
                    }
                    if (labelLength == 0)
                    {
                        return false;
                    }
                    return true;
                }
                internal static bool IsAlpha(byte value) {
                    return(value >= NumericUnchecked.ToByte('A') && value <= NumericUnchecked.ToByte('Z')) || (value >= NumericUnchecked.ToByte('a') && value <= NumericUnchecked.ToByte('z'));
                }
                internal static bool IsDigit(byte value) {
                    return value >= NumericUnchecked.ToByte('0') && value <= NumericUnchecked.ToByte('9');
                }
                internal static bool IsSchemeChar(byte value) {
                    return IsAlpha(value) || IsDigit(value) || value == NumericUnchecked.ToByte('+') || value == NumericUnchecked.ToByte('-') || value == NumericUnchecked.ToByte('.');
                }
                internal static bool EqualsAscii(string left, string right) {
                    if (left == null || right == null)
                    {
                        return false;
                    }
                    if (left.Length != right.Length)
                    {
                        return false;
                    }
                    var index = 0;
                    while (index <left.Length)
                    {
                        let l = ToLowerAsciiChar(left[index]);
                        let r = ToLowerAsciiChar(right[index]);
                        if (l != r)
                        {
                            return false;
                        }
                        index += 1;
                    }
                    return true;
                }
                internal static char ToLowerAsciiChar(char value) {
                    if (value >= 'A' && value <= 'Z')
                    {
                        return NumericUnchecked.ToChar(NumericUnchecked.ToInt32(value) + 32);
                    }
                    return value;
                }
                internal static bool IsLoopbackHost(string host, UriHostNameType hostType, bool isFile) {
                    if (host == null || host.Length == 0)
                    {
                        return isFile;
                    }
                    if (hostType == UriHostNameType.IPv6)
                    {
                        return EqualsAscii(host, "::1");
                    }
                    if (hostType == UriHostNameType.IPv4)
                    {
                        return EqualsAscii(host, "127.0.0.1");
                    }
                    return EqualsAscii(host, "localhost");
                }
                internal static bool EqualsAuthority(Uri left, Uri right) {
                    if (! EqualsAscii (left._scheme, right._scheme))
                    {
                        return false;
                    }
                    let leftHost = HostForCompare(left);
                    let rightHost = HostForCompare(right);
                    if (! EqualsAscii (leftHost, rightHost))
                    {
                        return false;
                    }
                    let leftPort = left.Port;
                    let rightPort = right.Port;
                    return leftPort == rightPort;
                }
                internal static string HostForCompare(Uri uri) {
                    if (uri._hostNameType == UriHostNameType.IPv6 || uri._hostNameType == UriHostNameType.IPv4)
                    {
                        return uri._host;
                    }
                    if (uri._idnHost != null && uri._idnHost.Length >0)
                    {
                        return uri._idnHost;
                    }
                    return uri._host;
                }
                internal static bool StartsWithOrdinal(string value, string prefix) {
                    if (value == null || prefix == null)
                    {
                        return false;
                    }
                    if (prefix.Length >value.Length)
                    {
                        return false;
                    }
                    var index = 0;
                    while (index <prefix.Length)
                    {
                        if (value[index] != prefix[index])
                        {
                            return false;
                        }
                        index += 1;
                    }
                    return true;
                }
                internal static int CompareStrings(string left, string right, StringComparison comparisonType) {
                    if (left == null && right == null)
                    {
                        return 0;
                    }
                    if (left == null)
                    {
                        return - 1;
                    }
                    if (right == null)
                    {
                        return 1;
                    }
                    let leftSpan = left.AsUtf8Span();
                    let rightSpan = right.AsUtf8Span();
                    var index = 0usize;
                    var minLength = leftSpan.Length <rightSpan.Length ?leftSpan.Length : rightSpan.Length;
                    while (index <minLength)
                    {
                        var l = leftSpan[index];
                        var r = rightSpan[index];
                        if (comparisonType == StringComparison.OrdinalIgnoreCase || comparisonType == StringComparison.CurrentCultureIgnoreCase || comparisonType == StringComparison.InvariantCultureIgnoreCase)
                        {
                            if (l >= NumericUnchecked.ToByte ('A') && l <= NumericUnchecked.ToByte ('Z'))
                            {
                                l = NumericUnchecked.ToByte(NumericUnchecked.ToInt32(l) + 32);
                            }
                            if (r >= NumericUnchecked.ToByte ('A') && r <= NumericUnchecked.ToByte ('Z'))
                            {
                                r = NumericUnchecked.ToByte(NumericUnchecked.ToInt32(r) + 32);
                            }
                        }
                        if (l <r)
                        {
                            return - 1;
                        }
                        if (l >r)
                        {
                            return 1;
                        }
                        index += 1;
                    }
                    if (leftSpan.Length == rightSpan.Length)
                    {
                        return 0;
                    }
                    return leftSpan.Length <rightSpan.Length ?- 1 : 1;
                }
                internal static bool IsDefaultPort(string scheme, int port) {
                    let defaultPort = DefaultPort(scheme);
                    return defaultPort >= 0 && port == defaultPort;
                }
                internal static int DefaultPort(string scheme) {
                    if (scheme == null || scheme.Length == 0)
                    {
                        return - 1;
                    }
                    if (EqualsAscii (scheme, UriSchemeHttp) || EqualsAscii (scheme, UriSchemeWs))
                    {
                        return 80;
                    }
                    if (EqualsAscii (scheme, UriSchemeHttps) || EqualsAscii (scheme, UriSchemeWss))
                    {
                        return 443;
                    }
                    if (EqualsAscii (scheme, UriSchemeFtp))
                    {
                        return 21;
                    }
                    return - 1;
                }
                internal static string BuildRelativePath(string basePath, string targetPath) {
                    let baseSpan = basePath.AsUtf8Span();
                    let targetSpan = targetPath.AsUtf8Span();
                    var baseEnd = baseSpan.Length;
                    if (baseEnd >0 && baseSpan[baseEnd - 1] != NumericUnchecked.ToByte ('/'))
                    {
                        var idx = baseEnd;
                        while (idx >0)
                        {
                            idx -= 1;
                            if (baseSpan[idx] == NumericUnchecked.ToByte ('/'))
                            {
                                baseEnd = idx + 1;
                                break;
                            }
                        }
                    }
                    var common = 0usize;
                    let max = baseEnd <targetSpan.Length ?baseEnd : targetSpan.Length;
                    while (common <max)
                    {
                        if (baseSpan[common] != targetSpan[common])
                        {
                            break;
                        }
                        common += 1;
                    }
                    while (common >0 && baseSpan[common - 1] != NumericUnchecked.ToByte ('/'))
                    {
                        common -= 1;
                    }
                    var buffer = FVec.WithCapacity <byte >(targetSpan.Length + 8);
                    var index = common;
                    while (index <baseEnd)
                    {
                        if (baseSpan[index] == NumericUnchecked.ToByte ('/'))
                        {
                            AppendString(ref buffer, "../");
                        }
                        index += 1;
                    }
                    var targetIndex = common;
                    while (targetIndex <targetSpan.Length)
                    {
                        FVec.Push <byte >(ref buffer, targetSpan[targetIndex]);
                        targetIndex += 1;
                    }
                    let result = Utf8String.FromSpan(FVec.AsReadOnlySpan <byte >(in buffer));
                    FVecIntrinsics.chic_rt_vec_drop(ref buffer);
                    return result;
                }
                }
            }
