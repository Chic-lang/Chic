namespace Exec;

import Std.Text.Json;

public struct Person
{
    public int Id;
    public string Name;
}

public static class Program
{
    public static int Main()
    {
        var options = new JsonSerializerOptions();
        var info = JsonTypeInfo<Person>.CreateObject(options);
        info.AddProperty("Id", (ref Person p) => p.Id, (ref Person p, int v) => p.Id = v);
        info.AddProperty("Name", (ref Person p) => p.Name, (ref Person p, string v) => p.Name = v);
        var ctx = new JsonSerializerContext(options);
        ctx.AddTypeInfo(info);
        options.TypeInfoResolver = ctx;

        var person = new Person();
        person.Id = 7;
        person.Name = "Nova";

        var json = JsonSerializer.Serialize(person, options);
        Std.Console.WriteLine(json);

        var clone = JsonSerializer.Deserialize<Person>(json, options);
        Std.Console.WriteLine(clone.Id.ToString() + ":" + clone.Name);
        return 0;
    }
}
