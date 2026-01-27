namespace Samples;

public delegate TResult Converter<in T, out TResult>(T value);

public int FromBase(Base b) => 1;
public int FromDerived(Derived d) => 2;

public int Main()
{
    let baseConv = (Converter<Base, int>)FromBase;
    let derivedConv = (Converter<Derived, int>)FromDerived;

    // covariance on TResult: Derived -> Base accepted
    let cov: Converter<Derived, int> = baseConv;
    // contravariance on T: Base <- Derived accepted
    let contra: Converter<Base, int> = derivedConv;

    return cov(new Derived()) + contra(new Derived());
}

public class Base {}
public class Derived : Base {}
