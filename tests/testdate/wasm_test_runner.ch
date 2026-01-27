namespace Demo;

testcase Passes()
{
    return;
}

testcase ReturnsValue()
{
    var total = 0;
    total += 7;
}

testcase DividesByZero()
{
    var value = 1;
    value /= 0;
}
