namespace Chic.Web;
import Std.Async;
/// <summary>Represents the terminal request handler signature for the pipeline.</summary>
public delegate Task RequestDelegate(HttpContext context);
