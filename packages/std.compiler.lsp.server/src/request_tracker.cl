namespace Std.Compiler.Lsp.Server;

public struct RequestTracker
{
    private PendingRequest[] _pending;
    private int _count;

    public init() {
        _pending = new PendingRequest[8];
        _count = 0;
    }

    public void Track(ref this, long id, string method) {
        if (_count >= _pending.Length)
        {
            let resized = new PendingRequest[_pending.Length * 2];
            var idx = 0;
            while (idx < _pending.Length)
            {
                resized[idx] = _pending[idx];
                idx += 1;
            }
            _pending = resized;
        }
        _pending[_count] = new PendingRequest(id, method);
        _count += 1;
    }

    public bool TryComplete(ref this, long id, out string method) {
        var idx = 0;
        while (idx < _count)
        {
            if (_pending[idx].Id == id)
            {
                let captured = _pending[idx].Method;
                method = "" + captured;
                _count -= 1;
                if (idx != _count)
                {
                    _pending[idx] = _pending[_count];
                }
                return true;
            }
            idx += 1;
        }
        method = "";
        return false;
    }
}
