namespace Std.Compiler.Lsp.Types;

public struct ServerInfo
{
    public string Name;
    public bool HasVersion;
    public string Version;

    public init(string name) {
        Name = name;
        HasVersion = false;
        Version = "";
    }

    public init(string name, string version) {
        Name = name;
        HasVersion = true;
        Version = version;
    }
}

