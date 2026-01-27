namespace Exec
{
    public int Main()
    {
        var total = 0;
        var index = 0;

        while (index < 6)
        {
            switch (index % 3)
            {
                case 0:
                    total += 3;
                    break;
                case 1:
                    total += 1;
                    break;
                default:
                    total += 2;
                    break;
            }

            index += 1;
        }

        if (total != 11)
        {
            return 1;
        }

        return 0;
    }
}
