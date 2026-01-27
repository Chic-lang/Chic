namespace Std.Distributed;
import Std.Core;
import Std.Numeric;
import Std.Testing;
public struct CapabilityToken
{
    public ulong Id;
    public ulong Nonce;
}
public struct RetryPolicy
{
    public uint Attempts;
    public ulong BackoffNs;
    public static RetryPolicy SingleAttempt() {
        var policy = CoreIntrinsics.DefaultValue <RetryPolicy >();
        policy.Attempts = 1u;
        policy.BackoffNs = 0ul;
        return policy;
    }
    public static RetryPolicy Linear(uint attempts, ulong backoffNs) {
        var policy = CoreIntrinsics.DefaultValue <RetryPolicy >();
        policy.Attempts = attempts;
        policy.BackoffNs = backoffNs;
        return policy;
    }
}
testcase Given_actor_system_loopback_receives_message_When_executed_Then_actor_system_loopback_receives_message()
{
    var sys = new ActorSystem();
    let cap = sys.Register();
    let policy = RetryPolicy.SingleAttempt();
    sys.Send(cap, "ping", policy);
    var message = CoreIntrinsics.DefaultValue <ActorMessage >();
    let ok = sys.TryRecv(out message);
    Assert.That(ok).IsTrue();
}
testcase Given_actor_system_loopback_message_from_id_When_executed_Then_actor_system_loopback_message_from_id()
{
    var sys = new ActorSystem();
    let cap = sys.Register();
    let policy = RetryPolicy.SingleAttempt();
    sys.Send(cap, "ping", policy);
    var message = CoreIntrinsics.DefaultValue <ActorMessage >();
    let _ = sys.TryRecv(out message);
    Assert.That(message.From.Id).IsEqualTo(cap.Id);
}
testcase Given_actor_system_loopback_message_payload_When_executed_Then_actor_system_loopback_message_payload()
{
    var sys = new ActorSystem();
    let cap = sys.Register();
    let policy = RetryPolicy.SingleAttempt();
    sys.Send(cap, "ping", policy);
    var message = CoreIntrinsics.DefaultValue <ActorMessage >();
    let _ = sys.TryRecv(out message);
    Assert.That(message.Payload).IsEqualTo("ping");
}
testcase Given_actor_system_loopback_message_attempt_zero_When_executed_Then_actor_system_loopback_message_attempt_zero()
{
    var sys = new ActorSystem();
    let cap = sys.Register();
    let policy = RetryPolicy.SingleAttempt();
    sys.Send(cap, "ping", policy);
    var message = CoreIntrinsics.DefaultValue <ActorMessage >();
    let _ = sys.TryRecv(out message);
    Assert.That(message.Attempt).IsEqualTo(0u);
}
testcase Given_actor_system_loopback_message_serialized_non_empty_When_executed_Then_actor_system_loopback_message_serialized_non_empty()
{
    var sys = new ActorSystem();
    let cap = sys.Register();
    let policy = RetryPolicy.SingleAttempt();
    sys.Send(cap, "ping", policy);
    var message = CoreIntrinsics.DefaultValue <ActorMessage >();
    let _ = sys.TryRecv(out message);
    Assert.That(message.Serialized.Length >0).IsTrue();
}
testcase Given_actor_send_log_has_expected_length_When_executed_Then_actor_send_log_has_expected_length()
{
    var sys = new ActorSystem();
    let cap = sys.Register();
    let policy = RetryPolicy.Linear(3u, 10ul);
    sys.Send(cap, "payload", policy);
    let log = sys.SendLog();
    Assert.That(log.Length).IsEqualTo(3usize);
}
testcase Given_actor_send_log_first_delivered_When_executed_Then_actor_send_log_first_delivered()
{
    var sys = new ActorSystem();
    let cap = sys.Register();
    let policy = RetryPolicy.Linear(3u, 10ul);
    sys.Send(cap, "payload", policy);
    let log = sys.SendLog();
    Assert.That(log[0usize].Delivered).IsTrue();
}
testcase Given_actor_send_log_second_backoff_When_executed_Then_actor_send_log_second_backoff()
{
    var sys = new ActorSystem();
    let cap = sys.Register();
    let policy = RetryPolicy.Linear(3u, 10ul);
    sys.Send(cap, "payload", policy);
    let log = sys.SendLog();
    Assert.That(log[1usize].BackoffNs).IsEqualTo(10ul);
}
testcase Given_actor_send_log_third_backoff_When_executed_Then_actor_send_log_third_backoff()
{
    var sys = new ActorSystem();
    let cap = sys.Register();
    let policy = RetryPolicy.Linear(3u, 10ul);
    sys.Send(cap, "payload", policy);
    let log = sys.SendLog();
    Assert.That(log[2usize].BackoffNs).IsEqualTo(20ul);
}
testcase Given_actor_unknown_capability_rejected_When_executed_Then_actor_unknown_capability_rejected()
{
    var sys = new ActorSystem();
    var bogus = new CapabilityToken();
    bogus.Id = 99ul;
    bogus.Nonce = 1ul;
    let policy = RetryPolicy.SingleAttempt();
    Assert.Throws <ActorError >(() => {
        sys.Send(bogus, "denied", policy);
    }
    );
}
public struct MessageHeader
{
    public CapabilityToken From;
    public uint Attempt;
}
public enum ActorErrorCode
{
    UnknownCapability, Serialization,
}
public class ActorError : Exception
{
    public ActorErrorCode Code;
    public init() : base() {
        Code = ActorErrorCode.UnknownCapability;
    }
}
public struct WireEnvelope
{
    public CapabilityToken From;
    public uint Attempt;
    public ulong BackoffNs;
    public byte[] Payload;
}
public struct ActorMessage
{
    public CapabilityToken From;
    public string Payload;
    public uint Attempt;
    public byte[] Serialized;
}
public struct SendRecord
{
    public uint Attempt;
    public ulong BackoffNs;
    public bool Delivered;
}
internal struct CapabilityEntry
{
    public ulong Id;
    public ulong Nonce;
}
/// <summary>Deterministic loopback actor runtime for native and WASM.</summary>
public class ActorSystem
{
    private CapabilityEntry[] _registry;
    private usize _registryCount;
    private ActorMessage[] _queue;
    private usize _queueCount;
    private usize _queueHead;
    private SendRecord[] _sendLog;
    private usize _sendLogCount;
    private ulong _nextCap;
    public init() {
        _registry = new CapabilityEntry[4];
        _queue = new ActorMessage[4];
        _sendLog = new SendRecord[4];
        _registryCount = 0usize;
        _queueCount = 0usize;
        _queueHead = 0usize;
        _sendLogCount = 0usize;
        _nextCap = 1ul;
    }
    public CapabilityToken Register() {
        let id = _nextCap;
        _nextCap = _nextCap + 1ul;
        let nonce = id ^ 0xA5A55A5A12345678ul;
        EnsureRegistryCapacity();
        _registry[_registryCount].Id = id;
        _registry[_registryCount].Nonce = nonce;
        _registryCount = _registryCount + 1usize;
        var cap = CoreIntrinsics.DefaultValue <CapabilityToken >();
        cap.Id = id;
        cap.Nonce = nonce;
        return cap;
    }
    public void Send(CapabilityToken from, string payload, RetryPolicy policy) {
        if (! Validate (from))
        {
            var error = new ActorError();
            error.Code = ActorErrorCode.UnknownCapability;
            error.Message = "unknown capability";
            throw error;
        }
        var attempts = policy.Attempts == 0u ?1u : policy.Attempts;
        var attemptIdx = 0u;
        while (attemptIdx <attempts)
        {
            let backoff = policy.BackoffNs * NumericUnchecked.ToUInt64(attemptIdx);
            RecordSend(attemptIdx, backoff, attemptIdx == 0u);
            if (attemptIdx == 0u)
            {
                EnqueueMessage(from, payload, attemptIdx, backoff);
            }
            attemptIdx = attemptIdx + 1u;
        }
    }
    public bool TryRecv(out ActorMessage message) {
        if (_queueHead >= _queueCount)
        {
            message = CoreIntrinsics.DefaultValue <ActorMessage >();
            return false;
        }
        message = _queue[_queueHead];
        _queueHead = _queueHead + 1usize;
        return true;
    }
    public SendRecord[] SendLog() {
        var copy = new SendRecord[_sendLogCount];
        var idx = 0usize;
        while (idx <_sendLogCount)
        {
            copy[idx] = _sendLog[idx];
            idx = idx + 1usize;
        }
        return copy;
    }
    private bool Validate(CapabilityToken cap) {
        var idx = 0usize;
        while (idx <_registryCount)
        {
            if (_registry[idx].Id == cap.Id && _registry[idx].Nonce == cap.Nonce)
            {
                return true;
            }
            idx = idx + 1usize;
        }
        return false;
    }
    private void EnqueueMessage(CapabilityToken from, string payload, uint attempt, ulong backoff) {
        EnsureQueueCapacity();
        var bytes = SerializePayload(payload);
        var envelope = CoreIntrinsics.DefaultValue <WireEnvelope >();
        envelope.From = from;
        envelope.Attempt = attempt;
        envelope.BackoffNs = backoff;
        envelope.Payload = bytes;
        _queue[_queueCount].From = from;
        _queue[_queueCount].Attempt = attempt;
        _queue[_queueCount].Payload = payload;
        _queue[_queueCount].Serialized = SerializeEnvelope(ref envelope);
        _queueCount = _queueCount + 1usize;
    }
    private void RecordSend(uint attempt, ulong backoffNs, bool delivered) {
        EnsureSendLogCapacity();
        _sendLog[_sendLogCount].Attempt = attempt;
        _sendLog[_sendLogCount].BackoffNs = backoffNs;
        _sendLog[_sendLogCount].Delivered = delivered;
        _sendLogCount = _sendLogCount + 1usize;
    }
    private byte[] SerializePayload(string payload) {
        var len = NumericUnchecked.ToInt32(payload.Length);
        var bytes = new byte[len];
        var idx = 0;
        while (idx <len)
        {
            bytes[idx] = NumericUnchecked.ToByte(payload[idx]);
            idx = idx + 1;
        }
        return bytes;
    }
    private byte[] SerializeEnvelope(ref WireEnvelope env) {
        // Simple deterministic serialization: id:nonce:attempt:backoff:payload-bytes (hex)
        var header = env.From.Id.ToString() + ":" + env.From.Nonce.ToString() + ":" + env.Attempt.ToString() + ":" + env.BackoffNs.ToString() + ":";
        let payloadHexLen = NumericUnchecked.ToInt32(env.Payload.Length * 2usize);
        var total = header.Length + payloadHexLen;
        var bytes = new byte[total];
        var idx = 0;
        while (idx <header.Length)
        {
            bytes[idx] = NumericUnchecked.ToByte(header[idx]);
            idx = idx + 1;
        }
        var payloadIdx = 0usize;
        while (payloadIdx <env.Payload.Length)
        {
            let value = env.Payload[payloadIdx];
            let hi = NumericUnchecked.ToByte((value >> 4) & 0xFu8);
            let lo = NumericUnchecked.ToByte(value & 0xFu8);
            bytes[idx] = HexDigit(hi);
            bytes[idx + 1] = HexDigit(lo);
            idx = idx + 2;
            payloadIdx = payloadIdx + 1usize;
        }
        return bytes;
    }
    private byte HexDigit(byte value) {
        if (value <10u8)
        {
            return NumericUnchecked.ToByte('0' + value);
        }
        return NumericUnchecked.ToByte('A' + (value - 10u8));
    }
    private void EnsureRegistryCapacity() {
        if (_registry == null)
        {
            _registry = new CapabilityEntry[4];
            _registryCount = 0usize;
            return;
        }
        if (_registryCount <NumericUnchecked.ToUSize (_registry.Length))
        {
            return;
        }
        let newLen = _registry.Length == 0 ?4usize : NumericUnchecked.ToUSize(_registry.Length * 2);
        var grown = new CapabilityEntry[newLen];
        var idx = 0usize;
        while (idx <_registry.Length)
        {
            grown[idx] = _registry[idx];
            idx = idx + 1usize;
        }
        _registry = grown;
    }
    private void EnsureQueueCapacity() {
        if (_queue == null)
        {
            _queue = new ActorMessage[4];
            _queueCount = 0usize;
            _queueHead = 0usize;
            return;
        }
        if (_queueCount <NumericUnchecked.ToUSize (_queue.Length))
        {
            return;
        }
        let newLen = _queue.Length == 0 ?4usize : NumericUnchecked.ToUSize(_queue.Length * 2);
        var grown = new ActorMessage[newLen];
        var idx = 0usize;
        while (idx <_queue.Length)
        {
            grown[idx] = _queue[idx];
            idx = idx + 1usize;
        }
        _queue = grown;
    }
    private void EnsureSendLogCapacity() {
        if (_sendLog == null)
        {
            _sendLog = new SendRecord[4];
            _sendLogCount = 0usize;
            return;
        }
        if (_sendLogCount <NumericUnchecked.ToUSize (_sendLog.Length))
        {
            return;
        }
        let newLen = _sendLog.Length == 0 ?4usize : NumericUnchecked.ToUSize(_sendLog.Length * 2);
        var grown = new SendRecord[newLen];
        var idx = 0usize;
        while (idx <_sendLog.Length)
        {
            grown[idx] = _sendLog[idx];
            idx = idx + 1usize;
        }
        _sendLog = grown;
    }
}
