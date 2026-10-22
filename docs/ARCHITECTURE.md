# apkrunner — Arquitectura

> Estado: borrador v1 (2026-10-21)
> Objetivo del documento: fijar **qué** construimos, **qué no**, y **dónde cortamos la pila de Android**.

---

## 1. Objetivo

Ejecutar archivos `.apk` en Linux **como aplicaciones nativas**, sin contenedor Android,
sin máquina virtual y sin emulador, con una experiencia similar a Wine:

- `apkrunner run app.apk` abre la app en una ventana del escritorio.
- Se integra con el sistema: `.apk` con doble clic, icono, menú, archivos, portapapeles, notificaciones.
- Datos aislados por app, sin privilegios de root.
- Arranque rápido y consumo de RAM comparable al de una app nativa (no 1–2 GB de un Android completo).

## 2. No objetivos (por ahora)

| No objetivo | Motivo |
|---|---|
| Contenedor Android (Waydroid, Anbox, LXC) | Requiere un Android completo en paralelo; el usuario lo descartó explícitamente. |
| Emulador / VM (QEMU, Cuttlefish) | No es "nativo" y el coste de recursos es alto. |
| Google Play Services / Firebase | Propietario, depende de servicios de Google; solo stubs mínimos. |
| Apps con DRM fuerte (Netflix, banca) | Dependen de Widevine L1, SafetyNet/Play Integrity, SELinux. Fuera de alcance. |
| Compatibilidad 100% | Meta realista: apps simples y juegos 2D primero, cobertura amplia después. |

## 3. El problema real

Wine funciona porque Win32 es una **API de usuario** sobre las mismas syscalls de Linux:
se reimplementan `kernel32`/`user32`/`ntdll` y el resto es el kernel.

Una app Android en cambio depende de **toda la pila de usuarios de Android**:

```
APK
 └─ bytecode DEX ──► ART (intérprete/JIT/AOT + GC)
      └─ android.* (framework Java) ──► libcore (java.*) ──► bionic libc
           └─ JNI ──► libs nativas del APK ──► libandroid / liblog / EGL / OpenSLES
                └─ Binder ──► system_server ──► SurfaceFlinger / AudioFlinger / PackageManager
                     └─ kernel: binder, ashmem/memfd, dma-buf
```

Reimplementar eso "desde cero" es reescribir media Android: años, no semanas.

**Pero no hace falta reimplementarlo todo.** La clave es elegir bien el punto de corte.

## 4. El corte quirúrgico (decisión)

Cortamos **justo entre el APK y las APIs Java de Android**:

```
┌────────────────────────────────────────────────────────────┐
│ APK (bytecode + recursos + .so nativas)                     │
├────────────────────────────────────────────────────────────┤
│ CORTE  ──►  api-impl: android.* reimplementado (Java)       │
│             + shims nativos (Rust/C) para JNI/EGL/audio      │
├────────────────────────────────────────────────────────────┤
│ ART + libcore  (reutilizados, compilados para glibc)        │
├────────────────────────────────────────────────────────────┤
│ Linux: Mesa/EGL, GTK4/Wayland, PipeWire/ALSA, VFS            │
└────────────────────────────────────────────────────────────┘
```

Consecuencias:

1. **No escribimos una VM de bytecode.** Reutilizamos ART (intérprete + JIT + GC + `dex2oat`).
   Esto elimina el trozo más grande y más difícil. El `src/vm/` actual del repo se descarta.
2. **La capa `android.*` se escribe en Java, no en Rust.** Las apps llaman a esas clases por
   nombre, con reflexión y firmas JNI; tienen que existir como clases Java reales.
   Rust no puede sustituirlas (podría generar stubs, pero no es práctico).
3. **Rust es el pegamento del sistema**, donde Java es un mal lenguaje: CLI, parseo de APK,
   supervisor de procesos, shims nativos, integración con el escritorio, sandbox.
4. **Las `.so` nativas del APK se conservan** y se enlazan contra shims (`libandroid`,
   `liblog`, `libEGL`, `libGLESv2`, `libOpenSLES`). Esto es el rol de `bionic_translation`.
5. **No hay Binder ni system_server.** Los servicios que la app necesita (`ActivityManager`,
   `PackageManager`, `ContentResolver`, `WindowManager`) se implementan como objetos Java
   dentro del mismo proceso.

## 5. Piezas reutilizables (no reinventar)

Proyectos existentes que resuelven partes exactas del problema:

| Componente | Qué resuelve | Uso |
|---|---|---|
| **ATL** (`android_translation_layer`) | Implementación de `android.*` en Java + shims JNI + ventana GTK4 | Referencia y, en Fase A, motor |
| **art_standalone** | ART compilado para glibc/Linux (sin Android completo) | Motor de ejecución DEX |
| **bionic_translation** | Ejecutar `.so` compiladas contra bionic dentro de un proceso glibc | Libs nativas de los APK |
| **libandroidfw-standalone** | Lectura de `resources.arsc`, `AndroidManifest.xml` binario, assets | Parseo de APK |
| **libOpenSLES-standalone** | Audio de apps que usan OpenSL ES | Audio |
| Mesa / GTK4 / PipeWire | GPU, ventanas, audio | Host |

Referencias verificadas (2026-10-21):

- ATL: <https://gitlab.com/android_translation_layer/android_translation_layer> — GPLv3, activo, financiado por NLnet (NGI Mobifree).
- Build de ATL: ART + libcore de AOSP modificados para usar librerías del sistema; **no requiere imagen de Android**.
- Componentes: `art_standalone`, `bionic_translation`, `libandroidfw-standalone`, `libopensles-standalone`.
- Alpine los empaqueta en `edge/testing`. En Arch/AUR **no hay paquete** (verificado hoy).

## 6. Opciones evaluadas

| # | Estrategia | Sin contenedor | Esfuerzo hasta 1 app real | Cobertura |
|---|---|---|---|---|
| 1 | Contenedor Android (Waydroid) | ❌ | días | casi total |
| 2 | **Traducción híbrida** (ART + api-impl + shims biónicos) | ✅ | **semanas** | media, creciente |
| 3 | Reimplementar ART + framework desde cero en Rust | ✅ | años | baja |
| 4 | DEX → bytecode JVM (dex2jar) sobre OpenJDK | ✅ | semanas | muy baja: sin JNI, sin GLES, sin recursos |
| 5 | VM/emulador (QEMU + Android-x86) | ❌ | días | casi total |

**Decisión: opción 2.** Es la única que cumple "sin contenedor" y "semanas".
La opción 3 se conserva solo como banco de pruebas educativo (ver `src/vm/`), nunca como motor principal.
La opción 4 se descarta: rompe apps con código nativo y gráficos, que son la mayoría de las interesantes.

### 6.1 Dos variantes de la opción 2

**Variante A — apkrunner como capa de producto sobre ATL (recomendada para empezar).**

- apkrunner (Rust) = CLI + gestor de APKs + sandbox + integración de escritorio.
- El motor es ATL, invocado como proceso separado o embebido.
- Ventaja: apps reales en **semanas**. Desventaja: la compatibilidad depende de ATL y
  heredamos GPLv3.

**Variante B — runtime propio con las piezas de ATL.**

- Reutilizamos `art_standalone` + `bionic_translation` + `libandroidfw`, pero escribimos
  nuestro propio `api-impl` en Java y nuestros shims en Rust.
- Ventaja: control total, licencia propia, evolución independiente.
- Desventaja: volver a recorrer el camino de stubs de APIs; **meses** hasta paridad.

**Plan:** A primero para validar el producto y la UX; B en paralelo, reemplazando módulos
de ATL uno por uno detrás del mismo trait `RuntimeBackend`. El día que B alcance a A en
las apps objetivo, se cambia el backend por defecto.

### 6.2 Licencia (decidida)

**apkrunner es GPL-3.0-or-later.** ATL es GPLv3 y el proyecto lo reutiliza, así que se
adopta la misma licencia de forma explícita. No hay ambigüedad de "agregación" ni riesgo
legal al enlazar componentes.

## 7. Arquitectura de procesos

```
┌──────────────────────────────────────────────────────────────┐
│ apkrunner (Rust)                                             │
│  CLI · base de datos de apps · parser APK · sandbox · launcher│
└───────────────┬──────────────────────────────────────────────┘
                │ spawn + IPC (JSON por stdio / D-Bus)
┌───────────────▼──────────────────────────────────────────────┐
│ apkrunner-runtime (proceso por app)                          │
│  ├─ ART (libart) + libcore                                   │
│  ├─ api-impl.jar (android.*)                                 │
│  ├─ libtranslation_layer_main.so (JNI: ventana, input, etc.) │
│  ├─ libandroid.so / liblog.so / libEGL.so / libGLESv2.so     │
│  └─ .so nativas del APK (cargadas vía bionic_translation)    │
└───────────────┬──────────────────────────────────────────────┘
                │ GTK4 → Wayland/X11 · EGL → Mesa · PipeWire
┌───────────────▼──────────────────────────────────────────────┐
│ Escritorio Linux                                             │
└──────────────────────────────────────────────────────────────┘
```

Un proceso por app (aislamiento y cuelgues contenidos). El sandbox se aplica con
`bubblewrap` + `seccomp` + namespaces, sin root.

## 8. Rol de cada lenguaje

| Capa | Lenguaje | Por qué |
|---|---|---|
| CLI, config, DB, parseo APK, supervisor, IPC | **Rust** | Seguridad, binario único, ecosistema actual del repo |
| `android.*` (View, Activity, Context, PackageManager…) | **Java** → `.jar`/`.dex` | Deben ser clases reales para ART, reflexión y JNI |
| Shims JNI, `libandroid`, EGL/GLES, audio, input | **C/C++** (o Rust con `#[no_mangle]`) | ABI C, deben exponer símbolos ELF exactos |
| UI de escritorio | **GTK4** (vía Java/JNI) | Es lo que usa ATL; reutilizable y probado en Linux |

> **Corrección al repo actual:** el `VM` en Rust (`src/runtime/vm.rs`, `src/vm/`) **no es el camino**.
> Un intérprete DEX propio es un juguete educativo; ART ya resuelve eso mejor. Se archiva.

## 9. Layout propuesto del repositorio

```
apkrunner/
├── Cargo.toml                 # workspace
├── crates/
│   ├── apkrunner-cli/         # binario: run/install/list/remove/inspect
│   ├── apkrunner-apk/         # parser APK: zip, AndroidManifest binario, resources.arsc, ABIs
│   ├── apkrunner-core/        # trait RuntimeBackend, modelo de app, rutas XDG, DB (sqlite)
│   ├── apkrunner-runtime/     # supervisor: sandbox, lanzamiento, logs, IPC
│   └── apkrunner-desktop/     # .desktop, iconos, MIME, portal
├── runtime/
│   ├── api-impl/              # Java: android.* (fase B)
│   ├── jni/                   # C: shims nativos
│   └── build/                 # scripts de art_standalone / bionic_translation
├── docs/
│   ├── ARCHITECTURE.md
│   └── ROADMAP.md
└── tests/
    ├── apks/                  # fixtures pequeñas, con licencia compatible
    └── e2e/                   # lanzar app y verificar ventana/log
```

## 10. Rutas y datos (XDG)

```
~/.local/share/apkrunner/
├── apps/<package>/<version>/
│   ├── base.apk
│   ├── lib/<abi>/*.so
│   └── meta.json
├── data/<package>/            # equivale a /data/data/<package>
│   ├── files/  cache/  shared_prefs/  databases/
└── shared/                    # equivale a /sdcard (visible para el usuario)
```

Nada de `/tmp` fijo (error actual del repo). Todo bajo XDG, con `APKRUNNER_DATA_DIR` para tests.

## 11. Seguridad

- Cada app corre como el usuario, sin root, en un `bubblewrap` con su propio `/home` falso.
- Por defecto **sin red**; se habilita por app (`--network`).
- Solo ve su `data/<package>` y `shared/` (bind mounts).
- `seccomp` para bloquear syscalls peligrosas.
- Sin SELinux: no pretendemos replicar el modelo de Android; el sandbox lo reemplaza.

## 12. Compatibilidad: qué funciona y qué no

| Categoría | Fase inicial | Notas |
|---|---|---|
| Apps Java/Kotlin simples (calculadora, RSS, notas) | ✅ objetivo MVP | Sin `.so` propias |
| Juegos 2D con libGDX/SDL nativo | ⚠️ | Requiere shims EGL + audio + `libandroid` |
| Flutter / React Native / Unity | ❌ corto plazo | Muchas APIs + `.so` grandes |
| Apps arm64-only en host x86_64 | ❌ | Necesita traducción ARM (FEX/box64) — fase posterior |
| Apps con Play Services | ❌ | Solo stubs; muchas se niegan a arrancar |
| DRM / banca / anti-cheat | ❌ | Fuera de alcance |

## 13. Estado actual del código

**M0 completado (2026-10-21).** El esqueleto anterior (1.100 líneas, 30 archivos huérfanos,
un `parse_dex()` que devolvía vacío, tres desempaquetadores con zip-slip y un `Cargo.toml`
corrupto) fue archivado en el historial de git y reemplazado por un workspace:

```
crates/apkrunner-apk     ZIP, AXML binario, ABIs, frameworks, extracción segura
crates/apkrunner-core    rutas XDG, registro de apps, validación de paquetes
crates/apkrunner-cli     binario `apkrunner` (inspect/run/install/list/remove)
```

- `apkrunner inspect` funciona sobre APKs reales, sin `aapt` ni `unzip`.
- Parser AXML propio con tests que generan manifiestos binarios reales.
- `safe_join` rechaza zip-slip, rutas absolutas y symlinks.
- `cargo test` y `cargo clippy -D warnings` limpios.
- La VM propia (`runtime/vm.rs`, `src/vm/`) se descartó: ART es el motor.

## 14. Decisiones abiertas

1. ~~**Licencia**~~: decidida, GPL-3.0-or-later.
2. **Motor inicial**: ATL embebido, ATL como proceso, o solo sus piezas (`art_standalone` + `bionic_translation`) con api-impl propio (se decide en M1).
3. **UI**: GTK4 (reutiliza ATL) vs Wayland/`iced`/`slint` propio (más trabajo, más control).
4. **Host objetivo**: x86_64 primero (esta máquina) y ARM64 después, o ambos desde el inicio.
5. **Nombre**: `apkrunner` se mantiene como nombre del producto/CLI.
