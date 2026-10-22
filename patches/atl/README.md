# ATL compatibility patches

`scripts/build-runtime.sh` applies these patches, in filename order, to the
ATL checkout before compiling it. The script skips patches that are already
applied, so rebuilds are idempotent.

| Patch | Why |
| --- | --- |
| `0001-http-response-cache.patch` | Adds `android.net.http.HttpResponseCache`. Apps such as JW Library resolve it during `MainActivity.onCreate`; without it the process aborts with `ClassNotFoundException`. |
| `0002-android-gcm-iv.patch` | Makes AES/GCM use a 12-byte IV, like Android. BouncyCastle picks 16 bytes, while Tink copies only the first 12 (`Cipher.getIV()`), so every keystore-backed decryption fails with `AEADBadTagException`. |
| `0003-web-settings-fonts.patch` | Adds the hidden `WebSettings.set*FontFamily` methods used by WebView-based apps. They are no-ops for now. |
| `0004-webview-javascript-bridge.patch` | Implements `addJavascriptInterface`, `evaluateJavascript`, `javascript:` URLs and a WebKit `android-asset://` handler for local app assets. The injected bridge object is a `Proxy` that also answers `"method" in window.NativeInterface`, which is how JW Library checks for the bridge before calling it. |
| `0005-path-arc-to.patch` | Replaces `Path.arcTo`'s use of `gsk_path_builder_svg_arc_to` with a parametric cubic-Bézier conversion. GSK can spin forever on full or degenerate arcs; JW Library's vector splash triggered exactly that and blocked the GTK main loop. |

Check them against a new ATL revision with:

```sh
git -C /path/to/atl-src apply --check patches/atl/0001-http-response-cache.patch
git -C /path/to/atl-src apply --check patches/atl/0002-android-gcm-iv.patch
git -C /path/to/atl-src apply --check patches/atl/0003-web-settings-fonts.patch
git -C /path/to/atl-src apply --check patches/atl/0004-webview-javascript-bridge.patch
git -C /path/to/atl-src apply --check patches/atl/0005-path-arc-to.patch
```

The build script also re-dexes `core-oj` and `core-libart` with D8 after
installing ART. Those boot jars are normally built with `dx`, which leaves
Java 8 lambdas as `invokedynamic`; ATL ships only the compile-time
`LambdaMetafactory` stub (it returns `null`), so `java.time` and other libcore
classes fail at runtime with `Bootstrap method returned null`.
