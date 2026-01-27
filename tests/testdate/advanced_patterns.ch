namespace Exec
{
    public struct Record
    {
        public int A;
        public int B;
    }

    static int MatchRecord(Record record)
    {
        switch (record)
        {
            case Record { A: 1, B: var value } when value > 5:
                return value;
            default:
                return 0;
        }
    }

    static int MatchList(int[] values)
    {
        switch (values)
        {
            case [let head, ..tail] when head == 1 when tail.Length >= 2:
                return head + tail.Length + tail[0];
            case [1, 2]:
                return 2;
            case [1, .., 3]:
                return 3;
            default:
                return 0;
        }
    }

    public int Main()
    {
        var list = new int[] { 1, 4, 3 };
        var record = new Record { A = 1, B = 6 };
        return MatchList(list) + MatchRecord(record);
    }
}
