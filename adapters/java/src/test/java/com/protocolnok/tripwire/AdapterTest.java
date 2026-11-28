package com.protocolnok.tripwire;

import static org.junit.jupiter.api.Assertions.assertEquals;

import com.google.gson.Gson;
import com.google.gson.JsonObject;
import org.junit.jupiter.api.Test;

class AdapterTest {

  private static final Gson GSON = new Gson();

  private static JsonObject check(String json) {
    return GSON.fromJson(json, JsonObject.class);
  }

  @Test
  void unknownXdrTypeIsUnsupported() throws Exception {
    var r = Adapter.runCheck("SomethingNew", new byte[] {0, 0, 0, 0}, check("{\"kind\":\"roundtrip\"}"));
    assertEquals("unsupported", r.status());
  }

  @Test
  void truncatedBytesFailRatherThanThrowUncaught() throws Exception {
    var r = Adapter.runCheck("ScVal", new byte[] {0, 0}, check("{\"kind\":\"roundtrip\"}"));
    assertEquals("fail", r.status());
  }

  @Test
  void unknownCheckKindIsUnsupported() throws Exception {
    var r = Adapter.runCheck("ScVal", new byte[] {0, 0, 0, 0}, check("{\"kind\":\"nonsense\"}"));
    assertEquals("unsupported", r.status());
  }

  @Test
  void muxedIdInputIsHonestlyUnsupported() throws Exception {
    // No string-parsing surface exists in this SDK's MuxedAccount API; the adapter must say
    // so, not silently report a pass it never checked.
    var r = Adapter.runCheck("TransactionEnvelope", new byte[] {0, 0, 0, 0}, check("{\"kind\":\"muxed_id_input\"}"));
    assertEquals("unsupported", r.status());
  }
}
