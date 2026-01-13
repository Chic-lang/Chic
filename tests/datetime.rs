use assert_cmd::Command;
use predicates::prelude::*;
use std::env;
use tempfile::tempdir;

mod common;
use common::write_source;

fn run_chic(source: &str, name: &str) {
    if env::var("CHIC_ENABLE_STDLIB_TESTS")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
        == false
    {
        eprintln!(
            "skipping datetime stdlib integration (set CHIC_ENABLE_STDLIB_TESTS=1 to opt in)"
        );
        return;
    }

    let dir = tempdir().expect("tempdir");
    let path = dir.path().join(name);
    write_source(&path, source);
    for backend in ["llvm", "wasm"] {
        Command::cargo_bin("chic")
            .expect("binary")
            .arg("run")
            .arg(path.to_str().unwrap())
            .args(["--backend", backend])
            .assert()
            .success()
            .stdout(predicate::str::is_empty())
            .stderr(predicate::str::is_empty().or(predicate::str::contains(
                "warning: overriding the module target triple",
            )));
    }
}

#[test]
fn datetime_roundtrip_and_offsets() {
    let program = r#"
namespace DateTimeTests;

using Std;
import Std.Datetime;

public class Program
{
    public int Main()
    {
        Date date = Date.FromParts(2024, 6, 10);
        Time time = Time.FromParts(12, 0, 0, 123000000);
        DateTime dt = DateTime.FromParts(date, time, DateTimeKind.Utc);
        string iso = dt.ToString("yyyy-MM-ddTHH:mm:ss.fffffffK", InvariantDateTimeCulture.Instance);
        if (iso != "2024-06-10T12:00:00.1230000Z")
        {
            return 1;
        }

        DateTime parsed;
        if (!DateTimeHelpers.TryParseIso(iso, out parsed))
        {
            return 2;
        }
        if (DateTime.Compare(dt, parsed) != 0)
        {
            return 3;
        }

        ZoneOffset ny = TimeZones.ResolveOffset("America/New_York", dt);
        if (!ny.IsDst || ny.TotalOffsetMinutes != -240)
        {
            return 4;
        }

        DateTime nyLocal = dt.ToLocalTime("America/New_York");
        if (nyLocal.Kind != DateTimeKind.Local)
        {
            return 5;
        }

        Duration ninety = Duration.FromSeconds(90);
        DateTime advanced = dt.Add(ninety);
        Duration diff = advanced.Subtract(dt);
        if (diff.Ticks != ninety.Ticks)
        {
            return 6;
        }

        string dur = diff.ToString();
        if (dur == "")
        {
            return 7;
        }

        return 0;
    }
}
"#;

    run_chic(program, "datetime_roundtrip.cl");
}

#[test]
fn datetime_custom_parsing_and_tzdb_install() {
    let program = r#"
namespace DateTimeTests;

using Std;
import Std.Datetime;

public class Program
{
    public int Main()
    {
        string data = "Custom/Zone;120;0;1;1;0;0;0;1;1;0;0;0\n";
        if (!TimeZones.InstallFromData(data))
        {
            return 1;
        }

        DateTime parsed;
        if (!DateTimeHelpers.TryParseCustom("yyyy-MM-dd HH:mm:ssK", "2024-12-01 05:00:00+00:00", out parsed))
        {
            return 2;
        }

        ZoneOffset custom = TimeZones.ResolveOffset("Custom/Zone", parsed);
        if (custom.TotalOffsetMinutes != 120)
        {
            return 3;
        }

        string formatted = parsed.ToString("yyyy-MM-ddTHH:mm:ssK", InvariantDateTimeCulture.Instance);
        if (formatted != "2024-12-01T05:00:00+00:00")
        {
            return 4;
        }

        return 0;
    }
}
"#;

    run_chic(program, "datetime_custom.cl");
}
