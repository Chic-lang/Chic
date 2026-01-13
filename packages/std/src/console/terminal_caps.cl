namespace Std;
import Std.Core;
import Std.Platform.IO;
public struct TerminalCapabilities
{
    public bool InputIsTerminal;
    public bool OutputIsTerminal;
    public bool ErrorIsTerminal;
    public bool SupportsAnsi;
    public bool SupportsColor;
    public bool SupportsCursor;
    public bool SupportsClear;
    public bool SupportsReadKey;
    public bool SupportsSizing;
    public static TerminalCapabilities Detect() {
        var caps = CoreIntrinsics.DefaultValue <TerminalCapabilities >();
        caps.InputIsTerminal = Stdin.IsTerminal();
        caps.OutputIsTerminal = Stdout.IsTerminal();
        caps.ErrorIsTerminal = Stderr.IsTerminal();
        caps.SupportsAnsi = caps.OutputIsTerminal || caps.ErrorIsTerminal;
        caps.SupportsColor = caps.SupportsAnsi && ! ColorsDisabled();
        caps.SupportsCursor = caps.SupportsAnsi && caps.OutputIsTerminal;
        caps.SupportsClear = caps.SupportsAnsi && caps.OutputIsTerminal;
        caps.SupportsReadKey = false;
        caps.SupportsSizing = false;
        return caps;
    }
    private static bool ColorsDisabled() {
        let noColor = Std.Environment.GetEnvironmentVariable("NO_COLOR");
        if (noColor != null && noColor != "")
        {
            return true;
        }
        let chicNoColor = Std.Environment.GetEnvironmentVariable("CHIC_NO_COLOR");
        if (chicNoColor != null && chicNoColor != "")
        {
            return true;
        }
        return false;
    }
}
