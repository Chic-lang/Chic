namespace DispatchDemo;

public class Animal
{
    public virtual int Speak()
    {
        return 1;
    }

    public virtual int Chain()
    {
        return Speak() + 10;
    }
}

public class Dog : Animal
{
    public sealed override int Speak()
    {
        return 2;
    }

    public int SpeakAsBase()
    {
        return base.Speak();
    }

    public override int Chain()
    {
        return base.Chain() + 20;
    }
}

public int Main()
{
    var baseAnimal = new Animal();
    var dog = new Dog();
    Animal pointer = dog;
    if (baseAnimal.Speak() != 1)
    {
        return 11;
    }
    if (pointer.Speak() != 2)
    {
        return 12;
    }
    if (dog.SpeakAsBase() != 1)
    {
        return 13;
    }
    if (pointer.Chain() != 32)
    {
        return 14;
    }
    return 0;
}
