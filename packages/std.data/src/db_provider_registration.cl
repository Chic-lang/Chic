namespace Std.Data;
import Foundation.Collections;
internal struct DbProviderRegistration
{
    internal string Name;
    internal DbProviderFactory Factory;
    internal init(string name, DbProviderFactory factory) {
        Name = name;
        Factory = factory;
    }
}
