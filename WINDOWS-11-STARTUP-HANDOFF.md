# Windows 11 startup crash: investigation handoff

Date: 2026-11-17  
Repository: https://github.com/chanonss/kruboffice-desktop  
Issue: https://github.com/chanonss/kruboffice-desktop/issues/4  
Local repository inspected: `/Users/chanon/dev/KrubOffice/launcher`, commit `357885b` (main). The user requested an assessment only; no application code was changed.

## Assessment

The proposed DirectX 12 workaround is credible for the specific AMD OpenGL driver crash documented in issue #4. The reporter says that `WGPU_BACKEND=dx12` lets KrubOffice 0.4.0 start and run normally. This is evidence for that machine, not proof that every Windows 11 startup failure has the same cause or that this change will solve all of them. The issue provides one detailed crash report; no independent Windows reproduction was performed during this investigation.

## Reported evidence

- KrubOffice 0.4.0 on 64-bit Windows 11, with AMD Radeon 780M integrated graphics and NVIDIA RTX 5060 Laptop GPU.
- Windows Event Viewer reports access violation `0xc0000005` in AMD OpenGL driver `atio6axx.dll`, version `32.0.31033.1001`, during startup. The issue attributes it to `egui_wgpu` initialization/backend probing.
- The reporter states that `WGPU_BACKEND=dx12` avoids the crash. They also report that selecting the high-performance GPU for each executable avoids it on their hybrid laptop, including file-association launches: https://github.com/chanonss/kruboffice-desktop/issues/4#issuecomment-6905546238.
- The issue suggests defaulting to DX12/Vulkan on Windows, excluding OpenGL unless explicitly requested. The observed successful workaround specifically tests DX12, not DX12 plus Vulkan.

## Relevant code

- The launcher `src/main.rs` creates `eframe::NativeOptions` with default graphics settings and calls `eframe::run_native`. The same is true in tags `v0.4.0` and `v0.5.0`. The checked-out launcher uses eframe 0.36.2, whose `egui-wgpu` default derives backend selection from `WGPU_BACKEND` or its defaults. Therefore the launcher is a relevant remaining place to investigate.
- The repository's published `sources/` snapshot for KrubOffice 0.5.0 already includes Windows graphics configuration in the three component applications. `sources/khiankrub/apps/wordcraft/src/graphics.rs` and `sources/khitkrub/apps/gridcraft/src/graphics.rs` probe DX12 and normally select it alone when a suitable hardware adapter is found, but use OpenGL in some fallback cases. `sources/phutkrub/apps/deckcraft/src/graphics.rs` defaults to DX12. Their entry points apply these configurations on Windows and honor explicit backend overrides. This finding does not establish what was in the 0.4.0 binaries.
- The component code explains that the reported AMD crash can occur while creating the OpenGL backend, before adapter selection or ordinary Rust error handling. If confirmed, selecting a preferred GPU only after instance creation is too late; avoiding initialization of the problematic backend is the relevant mechanism.
- `latest.json` in the inspected checkout advertises KrubOffice 0.5.0. The launcher remains on default graphics configuration in that checkout. No claim is made that a 0.5.0 installer was independently tested on the reporter's machine.

## Suggested next steps

1. Collect Event Viewer faulting module, exception code, KrubOffice version, GPU and driver versions from affected users. Distinguish `atio6axx.dll` access violations from other causes of an app not opening.
2. Reproduce on an affected Windows 11 machine with the original installer. Compare normal launch against `WGPU_BACKEND=dx12`, including the launcher, each component, and opening associated document files. Record the actual process and outcome.
3. If reproduced, consider configuring the launcher’s Windows wgpu instance before creation to avoid OpenGL on a suitable DX12 system, while honoring an explicit `WGPU_BACKEND` override. Evaluate fallback behavior deliberately: forcing DX12 on every machine could break systems without usable DX12; falling back to GL can reintroduce the original crash on affected machines. Avoid assuming a process-level access violation can be recovered with a normal Rust error fallback.
4. Do not assume adding Vulkan is necessary or safe solely from this report. Evaluate it separately on representative drivers if proposed.
5. Test the packaged installer and direct file associations on the affected hybrid AMD/NVIDIA setup, other Windows 11 GPU/driver combinations, and Windows 10, including a machine without a suitable hardware DX12 adapter. Confirm the override and diagnostics behave as intended.

The lack of reported Windows 10 cases does not by itself establish that Windows 11 is the cause; GPU, driver, and configuration differences also need investigation.

## Fix (2026-11-19, owner: "Yes, fix the launcher")

The Start Center (`kruboffice.exe`) now configures its wgpu instance like the three apps: `src/graphics.rs` is KhianKrub's `apps/wordcraft/src/graphics.rs` (identical code; same eframe 0.36.2 and wgpu 30.0.1), applied in `src/main.rs` before `eframe::run_native` on Windows. Without `WGPU_BACKEND` it probes DirectX 12 and uses DirectX 12 alone when a hardware adapter meets eframe's limits, so the OpenGL backend (and AMD's `atio6axx.dll`) is never created; OpenGL only when DirectX 12 has no usable GPU or on 32-bit builds; DirectX 12 shaders with FXC (`WGPU_DX12_COMPILER` overrides). `WGPU_BACKEND` still wins. Vulkan was not added (the report only shows DirectX 12 working). New dependency on Windows: `pollster` 0.4 (already in the build through rfd).

Checks: fmt; clippy -D warnings and tests on macOS (the graphics module's 5 tests run on every platform). Not done: a Windows build here (`cargo check --target x86_64-pc-windows-msvc` stops at `ring`, which needs a Windows C compiler), and a test on an affected AMD + NVIDIA laptop. The release that carries this is Windows only. Not pushed.
