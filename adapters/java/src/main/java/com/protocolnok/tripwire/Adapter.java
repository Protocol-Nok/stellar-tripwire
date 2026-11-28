package com.protocolnok.tripwire;

import com.google.gson.Gson;
import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import java.io.IOException;
import java.math.BigInteger;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.Map;
import org.stellar.sdk.Asset;
import org.stellar.sdk.LiquidityPool;
import org.stellar.sdk.MuxedAccount;
import org.stellar.sdk.SignerKey;
import org.stellar.sdk.scval.Scv;
import org.stellar.sdk.xdr.ChangeTrustAsset;
import org.stellar.sdk.xdr.Operation;
import org.stellar.sdk.xdr.SCVal;
import org.stellar.sdk.xdr.TransactionEnvelope;
import org.stellar.sdk.xdr.XdrElement;

/**
 * stellar-tripwire adapter for network.lightsail:stellar-sdk (the maintained successor to
 * org.stellar:java-stellar-sdk; GitHub redirects stellar/java-stellar-sdk here).
 *
 * <pre>java -jar tripwire-adapter-java.jar &lt;corpus-dir&gt;</pre>
 *
 * Protocol: ../../docs/adapter-protocol.md. One JSON object per line on stdout.
 */
public final class Adapter {
  private Adapter() {}

  record Result(String status, String detail) {
    static Result pass() {
      return new Result("pass", "");
    }

    static Result pass(String detail) {
      return new Result("pass", detail);
    }

    static Result fail(String detail) {
      return new Result("fail", detail);
    }

    static Result unsupported(String detail) {
      return new Result("unsupported", detail);
    }
  }

  public static void main(String[] args) throws IOException {
    if (args.length < 1) {
      System.err.println("usage: adapter <corpus-dir>");
      System.exit(2);
    }
    Path dir = Path.of(args[0]);
    Gson gson = new Gson();

    System.out.println(
        gson.toJson(Map.of("adapter", "java", "version", sdkVersion())));

    JsonObject manifest =
        gson.fromJson(Files.readString(dir.resolve("manifest.json")), JsonObject.class);
    for (JsonElement entryEl : manifest.getAsJsonArray("entries")) {
      JsonObject entry = entryEl.getAsJsonObject();
      String id = entry.get("id").getAsString();
      String xdrType = entry.get("xdr_type").getAsString();
      byte[] raw = Files.readAllBytes(dir.resolve(entry.get("file").getAsString()));
      for (JsonElement checkEl : entry.getAsJsonArray("checks")) {
        JsonObject check = checkEl.getAsJsonObject();
        String kind = check.get("kind").getAsString();
        Result r;
        try {
          r = runCheck(xdrType, raw, check);
        } catch (Exception e) {
          r = Result.fail("threw " + e.getClass().getSimpleName() + ": " + e.getMessage());
        }
        JsonObject out = new JsonObject();
        out.addProperty("entry", id);
        out.addProperty("check", kind);
        out.addProperty("status", r.status());
        out.addProperty("detail", r.detail());
        System.out.println(gson.toJson(out));
      }
    }
  }

  private static String sdkVersion() {
    // stellar-sdk-6.0.0.jar embeds no pom.properties of its own, so there's nothing to read
    // the version back out of at runtime (Package.getImplementationVersion() returns null).
    // version.properties is filtered by Maven at build time from pom.xml instead -- see
    // pom.xml's <properties> comment.
    String v = "(version unknown)";
    try (var in = Adapter.class.getResourceAsStream("/version.properties")) {
      if (in != null) {
        var props = new java.util.Properties();
        props.load(in);
        v = props.getProperty("stellar-sdk.version", v);
      }
    } catch (IOException e) {
      // leave v as "(version unknown)"
    }
    return "network.lightsail:stellar-sdk " + v;
  }

  // Package-private (not private) on purpose: AdapterTest calls this directly, in the same
  // package, rather than going through main()'s stdin/stdout plumbing or reflection.
  static Result runCheck(String xdrType, byte[] raw, JsonObject check) throws Exception {
    String kind = check.get("kind").getAsString();
    return switch (kind) {
      case "roundtrip" -> checkRoundtrip(xdrType, raw);
      case "native_preserves_map_entries" -> checkNativePreservesMapEntries(raw, check);
      case "muxed_ids_roundtrip" -> checkMuxedIdsRoundtrip(raw, check);
      case "muxed_id_input" ->
          Result.unsupported(
              "MuxedAccount's constructor takes a java.math.BigInteger id, not a string; "
                  + "there is no string-parsing boundary in this SDK's API for this check to exercise");
      case "liquidity_pool_asset_order" -> checkLiquidityPoolAssetOrder(raw, check);
      case "signed_payload_strkey" -> checkSignedPayloadStrkey(raw, check);
      default -> Result.unsupported("check kind not implemented by this adapter");
    };
  }

  // -- roundtrip ---------------------------------------------------------

  private static XdrElement decode(String xdrType, byte[] raw) throws IOException {
    return switch (xdrType) {
      case "ScVal" -> SCVal.fromXdrByteArray(raw);
      case "TransactionEnvelope" -> TransactionEnvelope.fromXdrByteArray(raw);
      case "ChangeTrustAsset" -> ChangeTrustAsset.fromXdrByteArray(raw);
      default -> throw new IllegalArgumentException("unknown xdr_type " + xdrType);
    };
  }

  private static Result checkRoundtrip(String xdrType, byte[] raw) {
    try {
      XdrElement decoded = decode(xdrType, raw);
      byte[] again = decoded.toXdrByteArray();
      return Arrays.equals(again, raw) ? Result.pass() : Result.fail("decode then encode changed the bytes");
    } catch (IllegalArgumentException e) {
      return Result.unsupported(e.getMessage());
    } catch (Exception e) {
      return Result.fail("does not decode as " + xdrType + ": " + e.getMessage());
    }
  }

  // -- native_preserves_map_entries ---------------------------------------

  private static Result checkNativePreservesMapEntries(byte[] raw, JsonObject check)
      throws IOException {
    SCVal val = SCVal.fromXdrByteArray(raw);
    int expected = check.get("entries").getAsInt();
    Map<SCVal, SCVal> native_ = Scv.fromMap(val);
    return native_.size() >= expected
        ? Result.pass(
            "Scv.fromMap keeps map keys as typed SCVal objects (not collapsed to a native "
                + "String/Number key), so this SDK's design has no entry-collision surface here")
        : Result.fail(expected + " entries in the XDR, " + native_.size() + " after Scv.fromMap, and no error");
  }

  // -- muxed_ids_roundtrip -------------------------------------------------

  /** The source account, then each payment destination, in order -- muxed ones only. */
  private static List<org.stellar.sdk.xdr.MuxedAccount> muxedAccounts(TransactionEnvelope env) {
    org.stellar.sdk.xdr.Transaction tx = env.getV1().getTx();
    List<org.stellar.sdk.xdr.MuxedAccount> all = new ArrayList<>();
    all.add(tx.getSourceAccount());
    for (Operation op : tx.getOperations()) {
      Operation.OperationBody body = op.getBody();
      if (body.getPaymentOp() != null) {
        all.add(body.getPaymentOp().getDestination());
      }
    }
    List<org.stellar.sdk.xdr.MuxedAccount> muxedOnly = new ArrayList<>();
    for (org.stellar.sdk.xdr.MuxedAccount m : all) {
      if (m.getDiscriminant() == org.stellar.sdk.xdr.CryptoKeyType.KEY_TYPE_MUXED_ED25519) {
        muxedOnly.add(m);
      }
    }
    return muxedOnly;
  }

  private static Result checkMuxedIdsRoundtrip(byte[] raw, JsonObject check) throws IOException {
    TransactionEnvelope env = TransactionEnvelope.fromXdrByteArray(raw);
    List<org.stellar.sdk.xdr.MuxedAccount> found = muxedAccounts(env);
    JsonArray expectedIds = check.getAsJsonArray("ids");
    if (found.size() != expectedIds.size()) {
      return Result.fail(
          "found " + found.size() + " muxed accounts, expected " + expectedIds.size());
    }
    List<String> bad = new ArrayList<>();
    for (int i = 0; i < found.size(); i++) {
      MuxedAccount m = MuxedAccount.fromXdr(found.get(i));
      BigInteger id = m.getMuxedId();
      String expected = expectedIds.get(i).getAsString();
      if (id == null || !id.toString().equals(expected)) {
        bad.add("#" + i + ": expected " + expected + ", SDK says " + id);
      }
    }
    return bad.isEmpty() ? Result.pass() : Result.fail(String.join("; ", bad));
  }

  // -- liquidity_pool_asset_order ------------------------------------------

  private static Result checkLiquidityPoolAssetOrder(byte[] raw, JsonObject check)
      throws IOException {
    ChangeTrustAsset cta = ChangeTrustAsset.fromXdrByteArray(raw);
    var params = cta.getLiquidityPool().getConstantProduct();
    Asset assetA = Asset.fromXdr(params.getAssetA());
    Asset assetB = Asset.fromXdr(params.getAssetB());
    LiquidityPool pool;
    try {
      pool = new LiquidityPool(assetA, assetB);
    } catch (RuntimeException e) {
      return Result.fail("refused an asset pair the network accepts: " + e.getMessage());
    }
    String got = pool.getLiquidityPoolId();
    String expected = check.get("pool_id").getAsString();
    return got.equalsIgnoreCase(expected)
        ? Result.pass()
        : Result.fail("pool id " + got + ", expected " + expected);
  }

  // -- signed_payload_strkey ------------------------------------------------

  private static Result checkSignedPayloadStrkey(byte[] raw, JsonObject check) throws IOException {
    TransactionEnvelope env = TransactionEnvelope.fromXdrByteArray(raw);
    org.stellar.sdk.xdr.Transaction tx = env.getV1().getTx();
    var ops = tx.getOperations();
    if (ops.length == 0
        || ops[0].getBody().getSetOptionsOp() == null
        || ops[0].getBody().getSetOptionsOp().getSigner() == null) {
      return Result.fail("entry has no SetOptions signer");
    }
    org.stellar.sdk.xdr.SignerKey xdrKey = ops[0].getBody().getSetOptionsOp().getSigner().getKey();
    SignerKey key = SignerKey.fromXdr(xdrKey);
    String got = key.getEncodedSignerKey();
    String expected = check.get("strkey").getAsString();
    if (!got.equals(expected)) {
      return Result.fail("encoded " + got + ", expected " + expected);
    }
    SignerKey back = SignerKey.fromEncodedSignerKey(got);
    return Arrays.equals(back.toXdr().toXdrByteArray(), xdrKey.toXdrByteArray())
        ? Result.pass()
        : Result.fail("strkey did not decode back to the same signer key");
  }
}
