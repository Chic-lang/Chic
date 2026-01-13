namespace Std
{
    public enum UriKind
    {
        RelativeOrAbsolute = 0, Absolute = 1, Relative = 2,
    }
    public enum UriHostNameType
    {
        Unknown = 0, Basic = 1, Dns = 2, IPv4 = 3, IPv6 = 4,
    }
    public enum UriComponents
    {
        Scheme = 1, UserInfo = 2, Host = 4, Port = 8, Path = 16, Query = 32, Fragment = 64, StrongAuthority = 128, AbsoluteUri = 127, PathAndQuery = 48,
    }
    public enum UriFormat
    {
        UriEscaped = 1, Unescaped = 2, SafeUnescaped = 3,
    }
    public enum UriPartial
    {
        Scheme = 0, Authority = 1, Path = 2, Query = 3,
    }
    public enum UriCreationOptions
    {
        None = 0, DangerousDisablePathAndQueryCanonicalization = 1, DisableIriParsing = 2, EnableIriParsing = 4,
    }
    public enum StringComparison
    {
        CurrentCulture = 0, CurrentCultureIgnoreCase = 1, InvariantCulture = 2, InvariantCultureIgnoreCase = 3, Ordinal = 4, OrdinalIgnoreCase = 5,
    }
}
