namespace Std;
import Std.Numeric;
import Std.Strings;
/// <summary>
/// Lightweight version representation (major.minor[.build[.revision]]).
/// </summary>
public struct Version : IEquatable <Version >
{
    public int Major;
    public int Minor;
    public int Build;
    public int Revision;
    public init(int major, int minor) {
        Major = major;
        Minor = minor;
        Build = - 1;
        Revision = - 1;
    }
    public init(int major, int minor, int build) {
        Major = major;
        Minor = minor;
        Build = build;
        Revision = - 1;
    }
    public init(int major, int minor, int build, int revision) {
        Major = major;
        Minor = minor;
        Build = build;
        Revision = revision;
    }
    public bool Equals(Version other) {
        return Major == other.Major && Minor == other.Minor && Build == other.Build && Revision == other.Revision;
    }
    public override string ToString() {
        var writer = new Std.StringWriter();
        writer.Write(Major.ToString());
        writer.Write(".");
        writer.Write(Minor.ToString());
        if (Build >= 0)
        {
            writer.Write(".");
            writer.Write(Build.ToString());
        }
        if (Revision >= 0)
        {
            writer.Write(".");
            writer.Write(Revision.ToString());
        }
        let text = writer.ToString();
        writer.dispose();
        return text;
    }
}
