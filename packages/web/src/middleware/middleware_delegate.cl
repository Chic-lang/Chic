namespace Chic.Web;
import Std.Async;
/// <summary>Transforms the pipeline by wrapping the next request delegate.</summary>
public delegate RequestDelegate Middleware(RequestDelegate next);
