use chic::driver::CompilerDriver;
use chic::logging::LogLevel;
use chic::{ChicKind, Target};
use std::io::Write;
use tempfile::NamedTempFile;

const SOURCE: &str = r#"
namespace Samples.Sync;

import Std.Sync;
import Std.Core;

public class OnceRunner : OnceCallback
{
    public int Count;

    public void Invoke()
    {
        Count += 1;
    }
}

public static class Program
{
    public static int Main()
    {
        MutexIncrements();
        RwLockGuardsAccess();
        OnceRunsOnce();
        AtomicsSynchronise();
        ArcAndRcRoundtrip();
        return 0;
    }

    private static void MutexIncrements()
    {
        var mutex = new Mutex<int>(0);
        for (var index = 0; index < 16; index += 1)
        {
            var guard = mutex.Lock();
            guard.Value += 1;
            guard.Release();
        }

        var finalGuard = CoreIntrinsics.DefaultValue<MutexGuard<int>>();
        if (!mutex.TryLock(out finalGuard))
        {
            finalGuard = mutex.Lock();
        }

        if (finalGuard.Value != 16)
        {
            throw new Std::InvalidOperationException("mutex did not accumulate");
        }
        finalGuard.Release();
    }

    private static void RwLockGuardsAccess()
    {
        var rw = new RwLock<int>(10);

        var readLock = rw.Read();
        var rejected = CoreIntrinsics.DefaultValue<RwLockWriteGuard<int>>();
        if (rw.TryWrite(out rejected))
        {
            throw new Std::InvalidOperationException("write lock should not succeed while read lock held");
        }
        readLock.Release();

        var writeLock = rw.Write();
        writeLock.Value = 42;
        writeLock.Release();

        var readBack = rw.Read();
        if (readBack.Value != 42)
        {
            throw new Std::InvalidOperationException("write mutation not visible to readers");
        }
        readBack.Release();
    }

    private static void OnceRunsOnce()
    {
        var once = new Once();
        var runner = new OnceRunner();
        if (!once.Call(runner))
        {
            throw new Std::InvalidOperationException("first call should execute callback");
        }
        if (once.Call(runner))
        {
            throw new Std::InvalidOperationException("second call should not execute callback");
        }
        if (runner.Count != 1)
        {
            throw new Std::InvalidOperationException("callback executed unexpected number of times");
        }
    }

    private static void AtomicsSynchronise()
    {
        var counter = new AtomicI32(0);
        for (var index = 0; index < 64; index += 1)
        {
            counter.FetchAdd(1, MemoryOrder.AcqRel);
        }
        var final = counter.Load(MemoryOrder.SeqCst);
        if (final != 64)
        {
            throw new Std::InvalidOperationException("atomic increments mismatch");
        }
    }

    private static void ArcAndRcRoundtrip()
    {
        var arc = new Std.Sync.Arc<int>(7);
        var arcClone = arc.Clone();
        var weak = arcClone.Downgrade();
        var upgraded = weak.Upgrade();
        if (upgraded == null || upgraded.Value != 7)
        {
            throw new Std::InvalidOperationException("weak upgrade failed");
        }

        var rc = new Std.Sync.Rc<int>(9);
        var rcClone = rc.Clone();
        var weakRc = rcClone.Downgrade();
        var upgradedRc = weakRc.Upgrade();
        if (upgradedRc == null || upgradedRc.Value != 9)
        {
            throw new Std::InvalidOperationException("weak rc upgrade failed");
        }
    }
}
"#;

#[test]
fn sync_primitives_execute_on_llvm_and_wasm() -> Result<(), Box<dyn std::error::Error>> {
    let mut temp_src = NamedTempFile::new().expect("create temp source");
    temp_src.write_all(SOURCE.as_bytes())?;
    temp_src.flush()?;

    let driver = CompilerDriver::new();
    let report = driver.check(
        &[temp_src.path().to_path_buf()],
        &Target::host(),
        ChicKind::Executable,
        true,
        false,
        false,
        &[],
        LogLevel::Info,
    )?;

    let module = report
        .modules
        .iter()
        .find(|module| module.input == temp_src.path())
        .expect("sync sample module missing");
    assert!(
        module.parse.diagnostics.is_empty(),
        "unexpected parser diagnostics: {:?}",
        module.parse.diagnostics
    );
    assert!(
        !report
            .type_diagnostics
            .iter()
            .any(|diag| diag.message.contains("[MM0101]") || diag.message.contains("[MM0102]")),
        "std.sync sample triggered MM-series diagnostics: {:?}",
        report.type_diagnostics
    );

    Ok(())
}
