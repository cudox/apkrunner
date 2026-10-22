# apkrunner — Roadmap

> Estado: borrador v1 (2026-10-21)
> Acompaña a [ARCHITECTURE.md](ARCHITECTURE.md). Las duraciones son estimaciones para
> **1 desarrollador a tiempo parcial**; los criterios de aceptación son lo vinculante.

---

## Resumen

| Hito | Entrega | Duración | Estado |
|---|---|---|---|
| **M0** | Repo sano + CLI + `inspect` real | 1–2 semanas | ✅ hecho (2026-10-21) |
| **M1** | Primer APK ejecutándose sin contenedor | 2–4 semanas | funcional |
| **M2** | Integración de escritorio + sandbox | 2–4 semanas | |
| **M3** | Libs nativas x86_64 + GLES + audio | 4–8 semanas | |
| **M4** | Cobertura: 10 apps objetivo + métricas | 6–10 semanas | |
| **M5** | Runtime propio (paridad con ATL) | meses | |

**Primer objetivo honesto:** una app Java/Kotlin simple corriendo en una ventana en **~4–6 semanas**.
"La mayoría de APKs" es un objetivo de **M4/M5**, no de M1. Ninguna arquitectura sin contenedor
llega a eso en semanas; lo que sí se logra en semanas es *el camino* y la primera app real.

---

## M0 — Repo sano, CLI e `inspect` (1–2 semanas)

**Objetivo:** que `cargo run -- inspect app.apk` imprima información real y que la base del
código no tenga deuda estructural.

### Tareas

- [x] Convertir el repo en workspace Cargo (`crates/*`) o, como paso intermedio, un solo crate
      con módulos limpios.
- [x] Borrar/archivar: `src/vm/`, `src/runtime/vm.rs`, `src/runtime/dex/decoder.rs`,
      `src/runtime/class_resolver.rs`, `src/runtime/jni.rs`, `src/vm/method.rs`.
- [x] Eliminar los tres desempaquetadores; dejar uno solo, con **protección zip-slip**
      (rechazar rutas absolutas y `..`, y no seguir symlinks).
- [x] Arreglar `Cargo.toml` (primera línea corrupta, `id="c1dex"`).
- [x] Reemplazar `panic!` por errores (`anyhow`/`thiserror`) y códigos de salida.
- [x] CLI real con `clap`:
      `apkrunner inspect <apk>` · `run <apk>` · `install <apk>` · `list` · `remove <pkg>`.
      `run` acepta `--activity`, `--abi`, `--data-dir`, `--verbose`.
- [x] `apkrunner inspect`:
  - [x] listar entradas del ZIP y ABIs presentes;
  - [x] contar `classes*.dex`;
  - [x] parsear `AndroidManifest.xml` **binario** (sin `aapt`):
        package, versionName/Code, minSdk/targetSdk, `application`, activities,
        `android:exported`, intent-filters, `uses-feature`, permisos;
  - [~] extraer label/icono resolviendo `resources.arsc` (fase 1: label con fallback al nombre de paquete);
  - [x] salida `--json` estable para scripting y tests.
- [x] Tests unitarios con manifiestos binarios generados + tests de zip-slip.
- [x] `README.md` con build, uso y estado.

### Criterio de aceptación

```console
$ apkrunner inspect tests/apks/hello.apk
package:      com.example.hello
version:      1.0 (1)
sdk:          min 21 / target 34
abis:         arm64-v8a, x86_64
dex:          1 (classes.dex, 1.2 MB)
launchable:   com.example.hello.MainActivity
framework:    native
$ apkrunner inspect --json tests/apks/hello.apk | jq .package
"com.example.hello"
```

- `cargo test` verde, `cargo clippy -- -D warnings` sin warnings.
- **Pendiente menor (M0.1):** resolver `label`/`icon` cuando son referencias a
  `resources.arsc`. Hoy se leen literales; las referencias se muestran como
  `@0x7f... (unresolved)`. La extracción de iconos entra en M2.
- Ningún archivo huérfano: todo `mod` declarado y alcanzable desde `main.rs`.
- Sin dependencia de binarios externos (`aapt`, `unzip`).

**Riesgo principal:** parsear `AndroidManifest.xml` binario y `resources.arsc` a mano.
Mitigación: usar `libandroidfw-standalone` si se puede enlazar desde Rust (FFI), o el crate
`abxml`/similar; si no, implementar solo el subconjunto necesario (string pool + XML chunks)
y dejar `resources.arsc` para M2.

---

## M1 — Primera app ejecutándose (2–4 semanas)

**Objetivo:** `apkrunner run app.apk` abre una ventana con una app Java/Kotlin sin `.so` propias,
sin contenedor, sin VM.

### Estrategia

Usar el motor de ATL (variante A de la arquitectura) para no reescribir ART ni los shims:

- [x] `RuntimeBackend` trait en `apkrunner-core`:
      `probe()`, `launch(request)` → programa + args + env.
- [x] Backend `atl`: detecta el binario y arma la línea de comandos
      (activity, `-w`/`-h`, `ANDROID_APP_DATA_DIR`, `LD_LIBRARY_PATH`, EGL en X11).
- [x] Si ATL no está instalado: error claro con instrucciones de build y
      `APKRUNNER_ATL_BIN` para overrides.
- [x] Entorno por app: `~/.local/share/apkrunner/runtime/atl/<package>`.
- [x] Capturar stderr del runtime en `~/.local/state/apkrunner/<pkg>.log`.
- [x] `apkrunner run` elige la activity: `--activity` > manifest > error explicativo.
- [x] `scripts/build-runtime.sh` reproducible (wolfSSL JNI + bionic + ART + ATL).
- [x] Verificar el criterio de aceptación con apps reales (2026-10-21):
      `demo_app` (Java), `compose_demo_app` (Kotlin/Compose) y
      `AdaptiveIconDemo` (Java) abren ventana y llegan a `onResume`
      (`onWindowFocusChanged: true`). `AdaptiveIconDemo` arranca en 3.2 s.
      `gles3jni` (GLES3 + JNI) también corre.
- [x] Prueba con APK real (JW Library 15.9.2, 2026-10-21): la app arranca,
      extrae sus 28 librerías nativas (Realm, SQLite, sodium, JNI propios),
      ejecuta `MainActivity` y `SiloContainer`, muestra el splash y se mantiene
      viva sin crashes. Queda bloqueada en el puente JavaScript del WebView:
      la webapp llama a `NativeInterface.send(...)` y espera respuestas, pero
      ATL no implementa `addJavascriptInterface`, `evaluateJavascript` ni
      `loadUrl("javascript:...")`.
- [x] Parches de compatibilidad de ATL en `patches/atl/`:
      `HttpResponseCache`, IV de 12 bytes en AES/GCM y métodos ocultos de
      `WebSettings`. El build también re-dexea `core-oj`/`core-libart` con D8
      porque `dx` deja las lambdas de libcore como `invokedynamic` y el
      `LambdaMetafactory` de ATL es un stub que devuelve `null`.
- [ ] Implementar el puente JavaScript del WebView en ATL (o en un fork
      propio): exponer objetos Java a JS y ejecutar JS desde Java. Es lo que
      falta para que JW Library salga del splash.
- [ ] Confirmar input manual (click/teclado) en las apps de prueba.
- [ ] `sample_sdl2_app` hace SIGSEGV dentro de `libandroidfw`
      (`DiagMessage::Build`), pendiente de reportar/actualizar ATL.

### Criterio de aceptación

- 3 apps Java/Kotlin de prueba (sin `.so`, sin Play Services) abren ventana y responden a input.
- `apkrunner run` sale con código ≠ 0 y un mensaje accionable si la app no es compatible.
- El proceso de la app muere al cerrar la ventana; sin procesos zombis.
- Startup < 5 s en esta máquina (i7 CoffeeLake, 15 GB, Intel UHD).

**Riesgo principal:** dependencias de build de ATL (GTK4, ffmpeg, wolfSSL, webkitgtk, OpenXR).
Mitigación: documentar y scriptear el build; evaluar contenedor de build (toolbox/Alpine)
solo para *compilar*, nunca para ejecutar la app.

---

## M2 — Que se sienta nativo (2–4 semanas)

- [ ] `.apk` registrado como MIME (`application/vnd.android.package-archive`) y abierto con apkrunner.
- [ ] `install` copia el APK, extrae libs, genera `.desktop`, icono PNG y entrada de menú.
- [ ] `list`/`remove` con datos por app y confirmación.
- [ ] Sandbox: `bubblewrap` con `/home` falso, sin red salvo `--network`.
- [ ] Carpeta `shared/` montada como almacenamiento externo; importar/exportar archivos.
- [ ] Portapapeles bidireccional y notificaciones (portal XDG).
- [ ] Manejo de DPI/escala y tamaño de ventana inicial (`--width/--height`).
- [ ] `apkrunner doctor`: diagnostica runtime, GPU, audio, permisos, versión de ART.

### Criterio de aceptación

- Doble clic en un `.apk` del gestor de archivos → instala y abre.
- La app aparece en el menú de XFCE y se puede desanclar.
- Copiar texto entre app y editor del host funciona.
- Sin red por defecto (verificado con `strace`/`bwrap`).

---

## M3 — Nativas, gráficos y audio (4–8 semanas)

- [ ] Carga de `.so` del APK vía `bionic_translation` en el proceso del runtime.
- [ ] Shims: `libandroid.so` (ANativeWindow, looper, assets), `liblog.so`, `libEGL.so`, `libGLESv2.so`.
- [ ] EGL/GLES sobre Mesa del host (Intel iGPU) con `GtkGLArea`; fallback software.
- [ ] Audio: `libOpenSLES` / `AudioTrack` → PipeWire o ALSA.
- [ ] Input táctil simulado con mouse, teclado físico, gamepad (mapeo básico).
- [ ] Detección de ABI: aviso claro si el APK es solo ARM.

### Criterio de aceptación

- Un juego 2D x86_64 con SDL/libGDX corre a ≥ 30 FPS con audio.
- `apkrunner inspect` marca correctamente las libs nativas que faltan.

**Riesgo principal:** `.so` arm64 en host x86_64. No se resuelve aquí; ver M4.

---

## M4 — Cobertura y métricas (6–10 semanas)

- [ ] Batería de 10 apps objetivo con estados: arranca / usable / rota (y por qué).
- [ ] `apkrunner report` genera la tabla de compatibilidad automáticamente.
- [ ] Priorizar stubs/implementaciones por frecuencia de fallo (top 20 clases/métodos).
- [ ] Evaluar traducción ARM (FEX-Emu) para libs arm64 en x86_64; decidir si entra.
- [ ] Perfilado: RAM por app, tiempo de arranque, CPU en reposo.

### Criterio de aceptación

- ≥ 6 de 10 apps objetivo "usables" (definidas de antemano, sin Play Services).
- Regresión automatizada: cada app del set corre en CI con `xvfb`/headless y verifica ventana + log.

---

## M5 — Runtime propio (meses)

- [ ] `api-impl` propio en Java, empezando por las clases más usadas (View, TextView, ImageView,
      Activity, Context, PackageManager, Resources, Canvas).
- [ ] Sustituir ATL por el runtime propio detrás del mismo trait, app por app.
- [ ] Empaquetado: Flatpak con permisos mínimos.
- [ ] Contribuir upstream a ATL/`art_standalone` lo que sea genérico.

---

## Backlog técnico priorizado

1. Parser de `AndroidManifest.xml` binario (sin `aapt`).
2. `resources.arsc` para label/icono.
3. `RuntimeBackend` + backend ATL.
4. Sandbox `bubblewrap` + seccomp.
5. `.desktop`/MIME/iconos.
6. Shims EGL/GLES + `libandroid`.
7. Audio OpenSLES → PipeWire.
8. Traducción ARM (evaluación).
9. `api-impl` propio.

## Riesgos globales

| Riesgo | Impacto | Mitigación |
|---|---|---|
| ATL es joven y cambia rápido | Alto | Backend aislado + pin de versión + tests e2e |
| GPLv3 de ATL | Legal | Decidir licencia antes de M1 (ver ARCHITECTURE §6.2) |
| Apps arm64-only | Alto en x86_64 | Priorizar APKs x86_64; FEX en M4 |
| Build de ART/GTK pesado | Medio | Script de build reproducible; binarios precompilados |
| Expectativa "todos los APKs" | Muy alto | Comunicar matriz de compatibilidad desde el inicio |
| Sin X11 nativo en ATL (GTK4/EGL) | Medio | `GDK_DEBUG=gl-egl`, probar en sesión Wayland si hace falta |

## Definición de "MVP logrado"

1. `apkrunner install app.apk` + doble clic abre la app en una ventana.
2. Funciona sin contenedor ni VM, con ART real.
3. 3 apps Java/Kotlin usables y 1 juego 2D x86_64.
4. Datos aislados, sin red por defecto, sin root.
5. `apkrunner inspect` es confiable y sin dependencias externas.
