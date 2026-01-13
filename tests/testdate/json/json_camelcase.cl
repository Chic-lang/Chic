namespace Exec;

import Std.Text.Json;

public struct Project
{
    public int ProjectId;
    public string OwnerName;
}

public static class Program
{
    public static int Main()
    {
        var options = new JsonSerializerOptions();
        options.PropertyNamingPolicy = JsonNamingPolicy.CamelCase;
        var info = JsonTypeInfo<Project>.CreateObject(options);
        info.AddProperty("ProjectId", (ref Project p) => p.ProjectId, (ref Project p, int v) => p.ProjectId = v);
        info.AddProperty("OwnerName", (ref Project p) => p.OwnerName, (ref Project p, string v) => p.OwnerName = v);
        var ctx = new JsonSerializerContext(options);
        ctx.AddTypeInfo(info);
        options.TypeInfoResolver = ctx;

        var project = new Project();
        project.ProjectId = 99;
        project.OwnerName = "Delta";

        var json = JsonSerializer.Serialize(project, options);
        Std.Console.WriteLine(json);

        var parsed = JsonSerializer.Deserialize<Project>(json, options);
        Std.Console.WriteLine(parsed.ProjectId.ToString() + ":" + parsed.OwnerName);
        return 0;
    }
}
