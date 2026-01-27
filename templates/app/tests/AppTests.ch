namespace {{project_namespace}}.Tests;

import {{project_namespace}};

testcase AddsTwoNumbers()
{
    return Program.Add(2, 3) == 5;
}

testcase AddsNegativeNumbers()
{
    return Program.Add(-1, 4) == 3;
}
