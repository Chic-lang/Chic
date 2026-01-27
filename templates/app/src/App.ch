import Std.Span;
import Std.Strings;

namespace {{project_namespace}};

public static class Program
{
    public static int Main(string[] args)
    {
        var target = SelectTarget(args);
        Std.Console.Write("Hello from ");
        Std.Console.Write(target);
        Std.Console.WriteLine("!");
        return 0;
    }

    public static int Add(int left, int right)
    {
        return left + right;
    }

    public static int CountArgs(string[] args)
    {
        return args.Length;
    }

    public static string SelectTarget(string[] args)
    {
        if (args.Length > 0)
        {
            var first = args[0];
            if (first != null && first.Length != 0)
            {
                let slice = SpanIntrinsics.chic_rt_string_as_slice(& first);
                return SpanIntrinsics.chic_rt_string_from_slice(slice);
            }
        }
        let fallback = "Chic";
        let slice = SpanIntrinsics.chic_rt_string_as_slice(& fallback);
        return SpanIntrinsics.chic_rt_string_from_slice(slice);
    }
}
