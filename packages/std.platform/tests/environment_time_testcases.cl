namespace Std.Platform;
import Std.Testing;

testcase Given_environment_info_returns_basics_When_executed_Then_environment_info_returns_basics()
{
    let os = EnvironmentInfo.OsDescription();
    Assert.That(os).IsNotEqualTo("");
}

testcase Given_environment_info_architecture_non_empty_When_executed_Then_environment_info_architecture_non_empty()
{
    let arch = EnvironmentInfo.Architecture();
    Assert.That(arch).IsNotEqualTo("");
}

testcase Given_environment_info_working_directory_non_empty_When_executed_Then_environment_info_working_directory_non_empty()
{
    let cwd = EnvironmentInfo.WorkingDirectory();
    Assert.That(cwd).IsNotEqualTo("");
}

testcase Given_environment_info_process_id_non_zero_When_executed_Then_environment_info_process_id_non_zero()
{
    let pid = EnvironmentInfo.ProcessId();
    Assert.That(pid).IsNotEqualTo(0);
}

testcase Given_environment_info_newline_is_lf_When_executed_Then_environment_info_newline_is_lf()
{
    Assert.That(EnvironmentInfo.NewLine()).IsEqualTo("\n");
}

testcase Given_environment_variables_set_returns_true_When_executed_Then_environment_variables_set_returns_true()
{
    let key = "CHIC_TEST_ENV";
    let okSet = EnvironmentVariables.Set(key, "1");
    Assert.That(okSet).IsTrue();
    let _ = EnvironmentVariables.Remove(key);
}

testcase Given_environment_variables_get_returns_value_When_executed_Then_environment_variables_get_returns_value()
{
    let key = "CHIC_TEST_ENV";
    let _ = EnvironmentVariables.Set(key, "1");
    let value = EnvironmentVariables.Get(key);
    Assert.That(value == "1").IsTrue();
    let _ = EnvironmentVariables.Remove(key);
}

testcase Given_environment_variables_remove_returns_true_When_executed_Then_environment_variables_remove_returns_true()
{
    let key = "CHIC_TEST_ENV";
    let _ = EnvironmentVariables.Set(key, "1");
    let okRemove = EnvironmentVariables.Remove(key);
    Assert.That(okRemove).IsTrue();
}

testcase Given_environment_variables_removed_value_is_null_When_executed_Then_environment_variables_removed_value_is_null()
{
    let key = "CHIC_TEST_ENV";
    let _ = EnvironmentVariables.Set(key, "1");
    let _ = EnvironmentVariables.Remove(key);
    let removed = EnvironmentVariables.Get(key);
    Assert.That(removed == null).IsTrue();
}

testcase Given_time_monotonic_is_positive_When_executed_Then_time_monotonic_is_positive()
{
    let mono = Time.MonotonicNanoseconds();
    Assert.That(mono).IsNotEqualTo(0UL);
}

testcase Given_time_utc_is_positive_When_executed_Then_time_utc_is_positive()
{
    let utc = Time.UtcNanoseconds();
    Assert.That(utc).IsNotEqualTo(0UL);
}

testcase Given_time_sleep_zero_does_not_throw_When_executed_Then_time_sleep_zero_does_not_throw()
{
    var ok = true;
    try {
        Time.SleepMillis(0UL);
    }
    catch(Exception) {
        ok = false;
    }
    Assert.That(ok).IsTrue();
}
