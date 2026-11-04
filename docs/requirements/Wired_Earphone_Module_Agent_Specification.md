# EarKey Control Module

Implementation specification for an AI development agent

Research date 3 November 2026 | Updated 4 November 2026 | Specification version 1.1

Build EarKey Control, a configurable, recoverable Android root module that suppresses unwanted wired earphone button input while preserving listening, microphone use, connection detection, and the phone's own controls. Prioritize KernelSU, then Magisk, then APatch. Develop against Android 12 and later, and publish support claims only for combinations backed by evidence.

This document is the development brief. It contains no implementation code, shell commands, configuration examples, or workflow YAML. Identifiers, filenames, tool names, and interface requirements are included so a development agent can implement the project precisely. The module itself has not been built or tested by preparing this specification.

## 1 Agent mandate and delivery boundary

Treat the requirements in this document as the project acceptance criteria. Work in small, reviewable increments, retain evidence, and complete the exit criteria of one stage before starting dependent device changes. Research, compilation, simulated events, and hardware testing establish different kinds of evidence; report them separately.

The requested deliverable for the implementing agent is a public source repository and a publishable installable module, together with reproducible builds, documentation, release assets, and an honest compatibility matrix. Creating this specification does not authorize accessing a phone, changing its root installation, creating a public repository, or publishing a release. During implementation, use the maintainer's stated access and publication instructions and prepare concrete reviewable artifacts before any decision requiring their approval.

Do not promise support on every Android device or every way of obtaining root. Root access alone does not supply a module loader, uinput, a usable input event path, compatible SELinux permissions, or identical button semantics. The product should aim for broad compatibility, detect its prerequisites, and decline unsafe activation without disrupting normal input. A refused device is unsupported; successful installation alone is not compatibility.

Use MUST for release requirements and SHOULD for recommendations that need a documented reason to vary. Treat the architecture below as the recommended starting design, not as a claim that it has already worked on the reference phone. If measurements require a different design, record the evidence and an architecture decision before changing direction.

## 2 Confirmed requirements and compatibility contract

### Confirmed reference setup

| Item | Confirmed requirement or fact |
| --- | --- |
| First root platform | KernelSU |
| Next platforms | Magisk, then APatch |
| Initial phone | Redmi Note 10 Pro |
| Initial ROM | PixelOS custom ROM |
| Initial accessory | Xiaomi Type-C wired earphones |
| Android target | Android 12 and later, beginning with API level 31 |
| Configuration | Disable all earphone button controls or selected controls |
| Preservation | Audio playback, microphone, connection detection, and unrelated phone input |

The exact phone variant, PixelOS build and Android release, kernel, KernelSU implementation and versions, earphone model, USB identifiers, and event mappings are not yet measured. Obtain them during discovery. Do not infer a codename, USB vendor identifier, or key layout filename from the product names. The phone's age does not establish which KernelSU fork or kernel integration it runs.

### Release support levels

Maintain a matrix keyed by Android release and API, ROM build, kernel, root implementation and manager versions, ABI, earphone identity, selected backend, and configuration capabilities. Distinguish these support levels:

- Verified means the named combination passed the required physical tests on the released artifact or an identical build with a recorded digest.
- Experimental means some evidence exists but the full physical acceptance suite has not passed. Activation requires an explicit experimental choice.
- Design target means implementation and tests are planned. It must not be advertised as working.
- Unsupported means prerequisites are missing, identity is ambiguous, or preservation cannot be established. Leave the device untouched and explain why.

Android 12 and later is the intended platform range, not a promise about all future Android versions. Record the oldest and newest physically tested releases for each release. Add later Android releases only after validation. KernelSU forks, legacy root tools, and module managers supplied by other projects require their own entries.

Use one source tree and prefer one release ZIP with small runtime adapters. If a manager genuinely needs different packaging, produce separately named assets from the same reviewed commit and explain the reason. Do not ask users to replace their kernel or root manager merely to test this module.

## 3 Research findings that constrain the design

### Input identity and key layouts

Android key layout files translate Linux input codes and HID usages into Android keycodes and flags. Layout selection can depend on device identifiers or a sanitized device name and can search device, vendor, system, and data locations. AOSP explicitly advises against modifying Generic.kl. A global search and replacement of volume or media mappings can affect unrelated controls. [S01]

An Android keycode, a Linux key code, and a HID usage are different identifiers. Keep their namespaces separate in profiles, tests, diagnostics, and documentation. A numeric value copied from an Android API is not a Linux code for an evdev filter. [S02]

The current AOSP input implementation may turn an unmapped key into UNKNOWN and continue processing it. A mapping omission or replacement with UNKNOWN is therefore not proof of complete event consumption. Its effect also depends on key character maps and the precise framework version. Any layout backend must demonstrate behavior on the actual ROM. [S09, S10]

### USB earphone actions can share a signal

The Android USB headset specifications describe a center control that can act differently for playback, calls, and long presses. A separate assistant control is optional. Thus, a request to block assistant while preserving every center-button action cannot always be satisfied by dropping one raw code. [S03, S04]

For reference, the standards associate the following controls. These are research references, not measured properties of the user's Xiaomi earphones.

| Standard control | HID usage page and usage | Standard Linux code | Android mapping or behavior |
| --- | --- | --- | --- |
| Center control | Consumer 0x0C and 0x0CD | KEY_PLAYPAUSE | MEDIA_PLAY_PAUSE with context dependent behavior |
| Volume up | Consumer 0x0C and 0x0E9 | KEY_VOLUMEUP | VOLUME_UP |
| Volume down | Consumer 0x0C and 0x0EA | KEY_VOLUMEDOWN | VOLUME_DOWN |
| Dedicated voice control | Consumer 0x0C and 0x0CF | KEY_VOICECOMMAND | VOICE_ASSIST |

Source for the table is the Android USB headset specifications. Capture the actual events before enabling any of these mappings. [S03, S04]

### Kernel input filtering has explicit prerequisites

Reading evdev events is observation, not cancellation. Exclusive grabbing is needed to stop the normal consumer from receiving a selected event device; that grab covers the device rather than just a chosen key. uinput provides a way to create a virtual device through which allowed events can be forwarded. [S06, S07, S08]

Linux input is stateful. Press, release, repeat, synchronization boundaries, and queue overrun recovery all matter. A filter that drops a press but forwards its release, or loses synchronization, can create incorrect behavior. [S07]

### Root module platforms overlap but differ

KernelSU, Magisk, and APatch provide related module layouts, metadata, boot hooks, and installer conventions. Similar packaging does not eliminate differences in manager detection, lifecycle support, paths, or available interfaces. [S11, S14, S16]

Current KernelSU documentation says scripts and policy features can run without a metamodule, while system-directory mounting requires one. Its metamodule architecture must not be confused with ordinary script execution. [S11, S12]

Current APatch documentation also describes metamodules. Do not assume all APatch releases have the same mounting behavior, and do not copy a KernelSU dependency check into APatch without checking its installed version. [S16, S17]

KernelSU has an official module WebUI mechanism. Use it for the first configuration interface, while keeping policy and persistence independent of that manager. The common design must not require Zygisk or LSPosed. [S13, S11, S16]

## 4 Discovery before modifying input

### Record the exact environment

Begin with read-only collection on the actual phone. Record Android version and API level, full ROM build identifier, kernel release and build, supported executable ABIs, root manager and userspace version, root kernel version where exposed, and whether the root implementation is upstream KernelSU or a named fork. Check SELinux enforcing state and the contexts in which module services execute.

Record whether uinput is present and usable by the intended service domain, whether evdev nodes can be inspected and read, and whether another remapping module is installed. A successful root shell is insufficient evidence for the service's permissions. Test the real service context later.

Identify the accessory's actual connection path. The Type-C connector alone does not establish its input transport. Correlate USB enumeration, audio device state, sysfs parentage, and evdev devices. If the earphone uses a different or proprietary path, document it instead of forcing the USB HID model.

Keep full device identifiers private unless needed locally. Public evidence should omit serial numbers, account details, notification contents, contact information, and unrelated logs. Keep exact build versions when they are necessary to reproduce a result.

### Build an event and action inventory

1. With filtering absent, capture the inventory of input devices and capabilities, then repeat after inserting and removing the earphones. Use AOSP getevent and input-service diagnostics for raw events and mapping information. Observe audio-service state separately. [S05]
2. For each button, perform short press, sustained press, release, double press, and any manufacturer documented gesture. Capture complete event frames, symbolic Linux codes, numeric values with their base, HID scan data when available, and timestamps.
3. Repeat the relevant gestures during media playback, while paused, with the screen locked and off, and during incoming and ongoing test calls. Use a consenting test participant or a local test setup; no emergency numbers.
4. Correlate each physical gesture with the evdev node, Android key mapping, and observed action. Record whether firmware emits distinct codes or the framework derives several actions from one stream.
5. Repeat each mapping observation at least five times. Repeat insertion at least three times to expose unstable event numbering and multiple input collections.
6. Test the phone's physical volume and power keys, touch input, and a separate keyboard or other available input device. Their event paths must be distinguishable from the earphones.
7. Establish baseline audio, microphone recording, call routing, plug and unplug behavior, and normal battery idle behavior before installing an interception prototype.

Do not capture all keyboard input continuously. Limit event recording to the enrollment window and selected candidates. Diagnostic collection must have a visible stop control and a finite timeout.

### Required discovery outputs

Produce a sanitized environment report, a candidate-device report, a button-to-event-to-action matrix, and fixtures containing only relevant earphone events. Include the mapping and permissions evidence used to choose a backend. Mark unsupported or unobserved gestures explicitly.

Discovery is complete only when the agent can identify the earphone controls independently of transient event numbers and explain whether unrelated controls or connection switches share their node. If data is missing, ask for that measurement; do not invent a hardware profile.

## 5 Define what the configuration can actually block

### Support event groups and observed actions

Model a profile as an identity plus measured input controls. Offer an easy Block all preset and a Custom preset with independently supported groups. A group can contain multiple raw codes when several codes produce the same intended class of control.

| Configuration group | Candidate events to discover | Required user explanation |
| --- | --- | --- |
| Center button | Play pause, hook, or other measured center code | Blocking may also block call control and long press assistant |
| Volume up | Measured volume up code or usage | Applies only to this enrolled earphone |
| Volume down | Measured volume down code or usage | Applies only to this enrolled earphone |
| Dedicated assistant | Distinct measured voice or assistant code | Independent only if it has a separable signal |
| Track controls | Distinct next, previous, seek, or stop codes | Multi-click derived actions may belong to center group |
| Separate call controls | Distinct measured answer or hang-up code | Context-derived call behavior may belong to center group |
| Other verified controls | Explicitly enrolled codes and their labels | No broad unknown-key rule outside a verified device |

These are logical categories. Do not assume every earphone exposes every category, or that Linux has a code with the same name as an Android action. A Block all preset covers all verified button controls for the enrolled accessory, including controls on its separate verified input collections. It does not mean blocking every device in the system.

### Handle shared actions honestly

If short press playback and long press assistant share the center signal, show that relationship in the UI before activation. Do not expose two apparently independent toggles backed by the same raw drop rule. Show the extra effects of disabling the group.

The first stable release MUST provide all-control blocking and selective groups where the measured signals permit independent control. If the user wants a distinction that the selected backend cannot represent, show Unsupported for this profile and retain the prior valid policy. This is a product limitation that the README and release notes must disclose.

Gesture recognition is a separate optional feature. Holding or delaying raw events to distinguish single press, double press, and long press can change latency and framework interpretation. Before adding it, define gesture timing, call-state dependencies, state transitions, lock-screen behavior, and delayed-event semantics, then validate each targeted ROM. Do not describe it as universally equivalent to the original framework behavior.

A framework-specific semantic adapter may eventually separate more contextual actions, but it has a different compatibility burden. Keep it outside the baseline until it has an explicit dependency model, versioned tests, recovery, and maintainer acceptance. Do not silently add framework injection to fulfill an unavailable toggle.

## 6 Recommended architecture and backend selection

### Primary design

Start with a mountless native evdev filter launched through the root manager's ordinary module service hook. It should enroll only a verified earphone control device, intercept its input, suppress blocked events, and forward allowed events through a validated uinput device. This is an engineering recommendation because it avoids rewriting shared key layouts and supports runtime policy changes; it remains subject to the discovery and physical validation gates.

Separate the project into a pure policy engine, device discovery and identity checks, a native input worker, a small supervisor, a root-manager adapter, configuration storage, and a user interface. Keep manager-specific logic out of the event filtering engine.

The approved core language is Rust for device discovery, filtering, policy, supervision, and the command interface. Keep unsafe system-call wrappers small and audited. Build Android binaries using Cargo and a pinned Rust toolchain with the Android NDK. Use BusyBox ash shell scripts for installation, startup, and uninstallation. Evaluate libevdev only if needed through an audited interface; build any selected dependency reproducibly. [S06, S31]

### Backend decision table

| Backend | Eligible circumstances | Required limits |
| --- | --- | --- |
| Native grab with uinput forwarding | Verified dedicated earphone key node, successful permission and mapping probes | Primary candidate for live selective configuration |
| Native grab with no forwarding | Verified key-only node and all its controls intentionally blocked | Optional reduced mode if uinput is absent; cannot offer selective passthrough |
| Device-specific key layout overlay | Exact file resolution and scoping known, all unrelated behavior preserved in physical tests | Optional explicit backend; may need mounting dependency and reboot |
| Framework semantic adapter | A needed contextual distinction cannot be represented below the framework | Separate experimental project stage with explicit dependencies |
| No interception | Ambiguous node, shared essential input, missing permissions, or unverified semantics | Required safe outcome; report the reason |

Do not silently switch backends after a failure. Different backends change timing, dependencies, and guarantees. Present the available capability and require an intentional policy choice when functionality is reduced.

### Scope the first native release conservatively

The initial native backend MUST reject nodes that combine the candidate keys with touch, pointer, gamepad, essential phone controls, or connection switch state. Reject general keyboards even if they have media keys. USB bus identity and a headset-like name are useful evidence, not sufficient authorization to grab a device.

Profiles for shared analog headset nodes or other mixed devices require a separate preservation design. A whole-node grab can hide more than button input. Do not generalize a working USB earphone profile to a phone's internal headset or GPIO node.

The module cannot reliably disable a volume change implemented entirely inside earphone hardware, a proprietary firmware command outside the observed input path, or an app action unrelated to the blocked events. Pausing caused by audio route removal is different from a button trigger and is outside the default feature scope.

## 7 Native input worker requirements

### Device enrollment and identity

Construct a match using multiple measured attributes: input bus, vendor and product identity when present, device name, interface or collection, capability fingerprint, and parent relationship to the accessory. Record which attributes are stable across reconnects and boots. Treat event numbers, major and minor numbers, and USB port position as runtime details rather than the sole saved identity.

For a composite earphone, group only the control collections proven to belong to that accessory. Separate serial-specific matching from model-level matching and explain their different scope. Avoid collecting a serial if a non-sensitive combination is sufficient. If two identical accessories or collections cannot be distinguished safely, decline automatic activation and ask for enrollment refinement.

Use an explicit deny policy for phone power and volume devices, touchscreen, fingerprint, navigation, lid and headset switches, keyboards, mice, and other mixed devices. A user-supplied profile cannot override the basic essential-device protection in the first stable release.

Revalidate identity after opening the device, not only after directory enumeration. Avoid symlink substitution and node-reuse races. Keep a distinct identity for the module's virtual devices and always exclude them from discovery to prevent feedback loops.

### Acquire control without losing state

1. Load and validate an immutable policy snapshot, then confirm that the profile still matches the opened physical device.
2. Inspect current key state and wait for a neutral state before transitioning to interception. Bound this wait and remain inactive if the device is stuck.
3. For selective mode, construct a virtual device with only supported, verified capabilities. Verify the Android mapping and policy flags needed for allowed controls, including any HID scan data.
4. Establish a bounded readiness check that Android has recognized the virtual device. An arbitrary delay alone is not evidence of readiness.
5. Acquire the exclusive grab, drain or reconcile queued state, and recheck current physical state. If a key changed during the transition and consistent reconciliation cannot be proven, abandon the transition and retry from neutral.
6. Announce Active only after the whole acquisition succeeds. On failure, close resources, remove the virtual device, and preserve normal physical input.

Opening an evdev file and ignoring events will not block Android. Conversely, grabbing a node before the passthrough path is ready can lose allowed input. Test both mistakes explicitly. [S08]

### Filter complete event sequences

For every tracked physical key, remember whether its press was blocked or forwarded. Apply the same decision to its repeats and release even if a configuration change arrives in the middle. Account for HID scan metadata so a blocked key's scan data cannot be accidentally associated with a following allowed key.

Preserve ordering and frame synchronization for allowed input. Handle multiple keys in the same frame, overlapping presses, repeated events, interrupted reads, short or failed writes, malformed input, and device removal. Use ABI-correct kernel definitions for input event structures; do not hardcode one architecture's byte size. [S07, S18]

Choose and test one owner for key repeat. Forwarding source repeats while also enabling independent proxy or framework repeat can duplicate actions. Verify long-held allowed volume keys against the baseline, including screen-off behavior, and configure the proxy so each intended repeat produces one action.

On SYN_DROPPED, follow the kernel synchronization procedure, query current state, and reconcile forwarded keys. If correct recovery is impossible, release interception and report a fault rather than guessing. [S07]

Do not use a shell getevent pipeline as the production event filter or replay events using a high-level Android input-injection command. They do not provide the required exclusive interception, frame handling, and precise source behavior. Diagnostic utilities remain appropriate for discovery.

### Preserve Android mapping and device behavior

A virtual device can select a different Android layout or classification from the physical one. Merely replaying the same Linux numbers does not establish equivalent behavior. Test allowed volume, media, assistant, wake, and call behavior and inspect Android's resulting device mapping. [S01, S09]

Prefer a clearly marked virtual identity. If preserving behavior requires copying selected identity fields or installing a specific mapping, document the tradeoff and verify that discovery cannot match the proxy again. If an overlay becomes necessary, disclose the additional mounting prerequisite and reclassify the backend accordingly.

Do not modify the physical device's keymap as a convenient substitute for proxy mapping. Do not change USB driver binding, unload audio drivers, disable USB interfaces, or take control of audio nodes.

### Failure handling and supervision

Use a single active worker instance and a small supervisor that does not own or inherit grabbed device descriptors. Prevent descriptor inheritance across unrelated child processes. A worker crash should close its resources; verify experimentally that the original input path becomes usable again. A hung worker needs an independent supervisor timeout because an exclusive grab does not expire by itself.

Implement a health check tied to real event-loop progress, not a separate thread that can keep reporting healthy while forwarding is stuck. On a fault, stop the worker, clear any forwarded pressed state when possible, destroy the proxy, and release the physical node. Rate-limit restart attempts; after three failures in one boot session, latch the feature inactive until explicit recovery or a clean restart.

Do not indefinitely hold a grab while writing logs, waiting for UI requests, or retrying a failed output. Use bounded queues and limits. When overloaded, return control to the original device and expose the error.

Use event-driven waiting for input and device changes. Watch node removal and creation, rescan after notification races, and revalidate every reconnect. Never assume the replacement event node is the same accessory.

### Timing and security limits

This module is a convenience input filter, not a security boundary against a malicious USB device. A button press before interception becomes active can reach Android. Startup, hotplug, configuration transition, failure, and recovery can have short inactive windows. Measure and disclose them rather than claiming zero leakage.

Set initial project targets of idle CPU below 0.1 percent over a defined five-minute sample, worker memory below 15 MiB, and additional passthrough latency below 20 ms at the 95th percentile on the reference device. These are proposed acceptance budgets, not measured results. Specify the measurement method, baseline, and instrumentation overhead. Avoid a persistent wakelock or high-frequency polling; investigate every idle wake source.

## 8 Configuration and persistence contract

### Canonical policy model

Store one manager-independent, versioned policy in a private root-owned project directory under /data/adb. Use structured data parsed as data. Never source a configuration file as shell code and never evaluate profile contents. The exact filename is a project choice to document before implementation.

| Field or concept | Required behavior |
| --- | --- |
| Schema version | Validate and migrate explicitly; reject unsupported future schemas |
| Master enabled state | Fresh installation remains inactive until enrollment and explicit activation |
| Preset | Block all verified controls or Custom selected groups |
| Accessory profiles | Stable identity, capabilities, mapping evidence version, and enabled state |
| Blocked groups | Explicit list of supported groups for each selected profile |
| Backend | Requested backend and the independently reported effective backend |
| Unknown accessory policy | Ignore and offer enrollment; never seize automatically |
| Diagnostics | Off by default; bounded duration and size when enabled |
| Recovery latch | Persistent reason for deliberately remaining inactive after repeated faults |

Treat a missing or unknown field according to a documented schema rule. Reject conflicting presets, duplicate identities, invalid codes, excessive lists, unsupported groups, malformed text, and oversized files. Do not silently widen a profile or silently replace unsupported options with Block all.

### Apply changes transactionally

Validate a candidate policy without changing active behavior. Show its capability and collateral effects, write it atomically, and obtain acknowledgment from the worker for the exact generation it applied. Report requested, saved, and active policies separately.

Apply a new decision for a key only after the current press sequence finishes, or transition the entire device at a safe neutral boundary. Keep the last good active configuration if a new one fails validation. If there is no valid policy, remain inactive. A UI success message must not be based only on a successful file write.

Provide separate operations for pausing interception now, resuming a validated policy, disabling activation on future boots, resetting to a safe inactive state, and forgetting an accessory profile. Pausing must not require physical earphone buttons.

### Persistence and access control

Use private directory permissions and root-owned policy files. Avoid world-writable data, sockets, and logs. A control socket, if used, must validate peer credentials and restrict operations; filesystem permissions alone are not the entire protocol design. If an optional companion app is added later, specify and test its authentication instead of opening a local network server.

Separate durable user policy from generated state and temporary runtime files. Back up and validate a policy before migration. Document rollback behavior when an older module cannot read a newer schema. Never overwrite an existing valid user policy with installation defaults during an update.

A KernelSU configuration API may be used as an adapter after verifying availability, but it must not become a hidden dependency for Magisk or APatch. A WebView's localStorage must not be the sole durable policy store. [S13]

## 9 User interfaces and observable status

### Required configuration operations

Provide a manager-independent root command interface with operations for discovering candidates, enrolling an accessory, listing supported groups, viewing and validating policy, selecting presets, applying changes, pausing, resuming, resetting, and exporting sanitized diagnostics. Document the actual syntax in the implemented repository after it exists; no syntax is prescribed here.

For KernelSU, implement the approved WebUI stack using TypeScript compiled to JavaScript, HTML, and CSS. Use the documented webroot entry point and supported bridge APIs. Bundle all resources locally and pin dependencies. Preserve manager-applied WebUI permissions unless a tested requirement justifies changing them. [S13]

For Magisk, the same configuration functionality must remain available through the root command interface. Its Action control can show status and guide the user to supported configuration operations, but do not assume an official built-in Magisk WebUI. APatch may expose a compatible UI only after testing its actual manager interface; otherwise use the documented root command interface. A third-party WebUI host is an optional dependency, never a hidden prerequisite.

### Design the interface around capability

Show the enrolled accessory, selected profile, supported groups, effective policy, and one clear state: Inactive, Waiting for earphones, Discovering, Ready, Active, Unsupported, or Fault. Include a brief reason and a recovery action when activation is unavailable. Do not confuse a running daemon with active suppression.

The primary screen should offer a master control, Block all, Custom, and Pause now. Under Custom, show only usable independent controls and explain linked center-button effects. State whether a change takes effect immediately, after release, after reconnection, or after reboot.

Support screen readers, keyboard focus, text scaling to at least 200 percent, strong contrast, descriptive labels, and touch targets usable on a phone. Do not rely on color to communicate active state. Configuration and recovery must be possible without pressing the broken earphone buttons.

### Treat the privileged bridge as an attack surface

Accept only defined operations and validated arguments. Do not concatenate device names, imported profiles, or UI text into shell strings. Escape all displayed device metadata as text, block remote scripts and navigation, and keep the privileged bridge away from external content. Avoid CDNs, analytics, remote fonts, and automatic upload of diagnostics.

## 10 Module packaging and runtime adapters

### Stable metadata and package layout

The approved module name is EarKey Control, its stable module ID is earkey_control, and its GitHub repository name is earkey-control. Its description is Configurable blocking of wired earphone button actions. Preserve the ID across updates. Keep the human version and monotonically increasing integer versionCode synchronized with the release manifest. Package only built, reviewed runtime assets.

| Package item | Purpose and requirement |
| --- | --- |
| module.prop | Identity, version, author, concise description, and verified update metadata fields |
| customize.sh | Manager, Android, ABI, permissions, and prerequisite checks during installation |
| service.sh | Portable entry point for bounded startup and native supervision |
| action.sh | Optional manager action for useful status or a supported control operation |
| uninstall.sh | Clean module-owned runtime state and stop interception safely |
| skip_mount | Declare the baseline package has no system overlay requirement |
| Native binaries | ABI-labelled worker and control tools produced from the release commit |
| webroot resources | Offline KernelSU configuration interface and tested adapters |
| Default policy and schemas | Safe inactive defaults and explicit validation rules |
| sepolicy.rule | Include only if narrowly scoped rules are demonstrated necessary |
| License and notices | Project license and required third-party attributions |

Keep module.prop at the installable ZIP root. Do not distribute GitHub's automatic source archive as the flashable module. The latter must be produced by the packaging workflow with the correct directory level, file permissions, and line endings.

Use manager-supported installation conventions and a pinned upstream installer wrapper where needed for Magisk compatibility. Keep custom logic in the documented customization hook rather than inventing a privileged installer. Follow manager cleanup and abort conventions. Avoid an install.sh filename and custom update-binary behavior that conflict with Magisk guidance. [S14, S28]

### Detect platforms correctly

Use official installation environment markers and actual capability probes. KernelSU can expose Magisk compatibility version variables, so those variables alone do not prove Magisk is the active manager. Distinguish installed versions and forks in diagnostics. For APatch use its documented environment rather than identifying it by a Magisk-like directory. [S11, S16]

Support only one active root environment per installation. If conflicting signals cannot be resolved, stop installation with a precise explanation. Root access through an unrelated su implementation is not evidence that this ZIP has a working boot lifecycle.

### Boot behavior

Launch the ordinary worker through the non-blocking service stage. Do not wait for accessories or run a long event loop in post-fs-data. Resolve the module's own location from the running hook rather than hardcoding its installed directory. Keep the module's runtime files and services under its own ownership. [S11, S14, S16]

The first stable implementation should not need preinit, initrc injection, global boot scripts, kernel patches, Zygisk, framework replacement, audio-policy edits, property spoofing, or permanent partition writes. Avoid these additional lifecycle paths unless a later, reviewed architecture decision demonstrates a necessity.

Handle manager-supported late loading or runtime installation only when explicitly tested. Otherwise report that a reboot is required. Do not claim full late-load support solely because the normal service hook exists.

### Installation and removal

Check Android API, available binary ABI, executable loading, required node access, and manager support before activation. Explain reduced all-control-only mode if uinput is unavailable and that mode has passed its own tests. Distinguish installable but inactive from fully supported.

Reject unsupported custom-recovery installation for the shared product. Root-manager installation inside a booted Android system is the supported route, consistent with KernelSU's documented limitation. Recovery may be used for removal when appropriate, not as an assumed installer. [S11]

Updates must not start a second worker, lose profile data, or reactivate a paused module. On uninstall, stop only the module's verified process instance, release grabs, destroy its virtual devices, and remove only its own files. Document whether users can choose to preserve settings; never leave an undisclosed service running outside the module.

Disabling a module in its manager may take effect at the next reboot. Provide an explicit Pause now operation for immediate release, and, where feasible, a tested worker check for its own disabled or removed state. Document the manager's real behavior rather than promising instant effect from every manager toggle.

## 11 Optional device specific key layout backend

Treat this as a separate, explicitly selected backend for a profile that cannot use the native path safely. Do not implement it first merely because editing text looks simpler.

Resolve the actual layout selected by Android, including any .idc configuration override, device identifiers, HID usage entries, and key character map effects. Check the exact framework and ROM sources when public documentation and runtime evidence differ. Do not assume a file placed under /system wins over a vendor or ODM file. [S01, S09, S29]

Preserve all unrelated mappings, policy flags, and axes. Scope any generated replacement to the intended input device and build from the relevant current device file or an audited complete mapping. Never edit Generic.kl globally, delete whole keylayout directories, or replace a shared internal key file without a preservation proof. [S01]

An empty layout, a commented mapping, UNKNOWN, or an invented keycode must not be described as a universal consume operation. Prove that the unwanted action is suppressed in the target contexts and that apps cannot still interpret the resulting event in a way that violates the documented feature. Otherwise leave the backend experimental. [S10]

Use systemless mounting through supported manager mechanisms. Detect and disclose the installed KernelSU or APatch mounting provider as appropriate. Do not auto-install a metamodule, replace an existing one, or bypass an unavailable provider with permanent writes. Explicitly document any limitation in mounting ODM or other partitions. [S12, S17]

Generate updates atomically and detect other modules touching the same effective file. Revalidate after a ROM OTA or layout digest change. Explain whether a reboot or accessory reconnection is required; mounting a file after Android has loaded it does not prove that it reloaded the mapping. Removal must restore the original view without modifying the underlying partition.

## 12 SELinux and privileged operation

Keep SELinux enforcing. Probe the actual module service domain, input node labels, uinput label, and diagnostic denials. Root UID does not guarantee access through SELinux. The agent must collect the minimum evidence necessary to explain a denial before proposing policy changes. [S30]

Start with no additional policy. If access is denied, define and test only the specific permissions needed for the selected domain and object types, using supported policy mechanisms. Document why each rule is necessary and on which environments it was tested. A rule accepted by an installer is not proof that the runtime can perform every required operation.

Do not make a domain or the system permissive, add wildcard allow rules, globally relabel input nodes, or grant broad access to unrelated devices. If the required operation cannot be enabled within the project's preservation and privilege constraints, leave filtering inactive on that environment.

Do not expose a TCP control service. Keep code and writable configuration separate, validate file ownership and expected file type, and prevent symlink and path traversal attacks during installation, migration, logging, and cleanup. Ignore untrusted profile paths outside the module's controlled directories.

Normal runtime must work offline. Installation and boot must not download executable payloads or run scripts obtained from a network endpoint. All executable components belong in the reviewed release ZIP.

## 13 Recovery and diagnostics

### Recovery contract

Preserve the phone's physical volume and power keys at every stage, including their role in root-manager rescue. Add a clearly documented module-specific way to remain inactive on the next boot. The escape path must be checked before opening any candidate input device.

Verify immediate pause, worker crash, worker hang, unplug, and reboot recovery on the reference phone. Check that allowed or forwarded keys cannot remain pressed. The supervisor and root-manager disable path are different layers and both need tests.

Prepare a recovery guide before the first interception build is installed. It must describe normal pause and removal, the root manager's current safe-mode procedure, and disabling this module from an available privileged recovery or debugging environment. State that recovery access and data decryption vary by device and must be checked in advance.

KernelSU documents a volume-down safe-mode mechanism with timing and integration qualifications; Magisk and APatch publish their own recovery procedures. Link those official guides and verify the installed versions instead of presenting one universal key sequence. Prefer disabling this module to broad removal of every module. [S15, S19, S20]

Do not make data wipes, flashing unrelated images, deleting root-manager executables, or changing firmware part of routine module recovery. If recovery requires something outside the module's normal scope, stop and obtain a concrete device-specific plan from the maintainer.

### Diagnostic content and privacy

Collect module version and commit, environment versions, backend capabilities, selected device identity with sensitive fields removed, effective policy generation, state transitions, concise error reasons, and limited counters for blocked and forwarded groups. Include relevant permission denials only when requested for debugging.

Use bounded, rotated logs. Default logs must not record every key press, general keyboard input, raw audio, USB serials, contact data, or call contents. Diagnostic export should preview what will be shared and save a sanitized local report. Nothing is uploaded automatically.

Publish a bug-report template that asks for the minimal reproduction, exact environment, selected groups, observed result, and sanitized diagnostics. A user should not have to paste an unrestricted system log into a public issue.

## 14 Repository structure and Git discipline

### Repository baseline

Before implementation, inspect the working tree, current branch, remotes, and repository instructions. Preserve unrelated work. Use an isolated branch or worktree when needed. Create or modify only the repository selected by the maintainer.

Organize the repository so source, module packaging inputs, UI, host tests, device test documentation, build tooling, and release metadata have clear owners and purposes. Keep generated binaries and build outputs out of ordinary source commits unless a specific vendoring or distribution decision requires them.

Include README, LICENSE, CHANGELOG, CONTRIBUTING, SECURITY, a dependency-notice record, issue and pull-request templates, compatibility documentation, recovery documentation, and AGENTS.md. The license must be chosen before public publication and third-party licenses must be recorded for the actual dependency versions. Do not assume that linking to an upstream project replaces its redistribution requirements.

AGENTS.md should direct future agents to discover hardware first, preserve essential controls, use this specification's stage gates, distinguish test evidence, avoid global key-layout edits, keep SELinux enforcing, and update documentation with changes. It must not grant blanket authority to publish or install on devices.

### Branches and commits

Use a protected main branch and short feature or fix branches. Each issue should define one behavior, its preservation constraints, and its acceptance test. Each commit should leave a coherent state and explain the problem and resulting behavior in its message.

Do not commit secrets, personal diagnostics, phone backups, boot images, access tokens, unreviewed vendor files, or incidental build caches. Review the staged diff before every commit. Do not rewrite shared history or move a published release tag to replace an artifact.

Before a pull request, synchronize with the intended base, resolve conflicts on the feature branch, and rerun checks affected by the resolution. Do not discard someone else's work to make the tree clean. Use focused cleanup commits only when relevant to the behavior being changed.

### Pull requests and merge requirements

Every behavior change should pass through a pull request containing the problem, final behavior, scope, test evidence, physical environments tested, recovery impact, and compatibility changes. Link the issue and relevant design decision. A test that was not run must be labelled not run with its reason.

Protect main with required checks, restricted force pushes, and review where a qualified reviewer is available. Use unique stable job names so required checks are unambiguous. GitHub supports these policies, but availability varies by repository and account plan. [S21]

For a solo maintainer, require explicit checklist review and checks without configuring an impossible self-approval requirement. If no independent review occurred, disclose that fact for sensitive changes. Do not fabricate reviewer approval, hardware test results, or required status checks.

Prefer squash merges for small issue branches if that is the repository policy; preserve meaningful commits when they aid review. Remove obsolete feature branches after merging. A release must refer to an exact reviewed main commit, not a developer's uncommitted working directory.

## 15 Step by step implementation sequence

Each stage has a reviewable output and an exit criterion. Keep its evidence with the relevant issue or pull request. Run independent host checks in parallel when convenient, but keep input acquisition, release preparation, and dependent device experiments sequential.

### Stage 0 Establish the project contract

Create the repository baseline, threat and failure model, support levels, architecture decision records, and task list. Record the approved EarKey Control identity and Rust, BusyBox ash, and TypeScript with HTML and CSS stack. Confirm the source license, repository ownership, and release policy. Any change to the approved identity or stack requires a documented decision and maintainer agreement. Plan tests before privileged code.

Exit only when the requirements and initial safe architecture are reviewable. The first pull request should contain documentation and build conventions, with no interception enabled.

### Stage 1 Discover the reference device

Perform Section 4 on the Redmi Note 10 Pro running PixelOS and KernelSU. Capture the real earphone identity, event collections, Android mappings, SELinux context, and baseline preservation checks. Do not install a blocking prototype yet.

Exit only with a measured profile candidate and an explicit decision on whether the primary native backend is eligible. If it is not, document the reason and evaluate a scoped alternative before continuing.

### Stage 2 Implement policy and identity validation

Implement the versioned policy model, strict parser, capability-aware groups, private persistence, and conservative matcher. Keep real device access read-only. Create fixtures for legitimate earphones, ambiguous duplicates, mixed devices, keyboards with media keys, and the module's own proxy.

Exit only when invalid policies cannot widen the target, unknown devices remain untouched, and shared action labels are represented correctly. Submit a focused pull request for the model and tests.

### Stage 3 Build the native filter engine on the host

Implement event-state processing behind a testable input and output interface. Test complete press sequences, mixed frames, scan metadata, repeats, policy generation changes, malformed input, output backpressure, and synchronization recovery.

Exit only after meaningful deterministic tests, fuzzing of data parsers, and applicable sanitizer runs pass. Do not claim host tests prove Android mapping or physical compatibility.

### Stage 4 Prove physical passthrough

Cross-compile for the measured reference ABI using pinned tooling. On the phone, test an explicitly enabled temporary prototype on only the verified key-only earphone collection. Start with no suppression and a short supervised session.

Prove that allowed controls act once, retain their intended behavior, and do not disturb audio, microphone, routing, or phone input. Verify proxy readiness, mapping, cleanup, and immediate pause. Exit only when passthrough matches the baseline and recovery works.

### Stage 5 Add all control blocking and selective groups

Enable Block all for the enrolled accessory, then test each separable group independently and in combination. Validate the center-button collateral effects in media and calls. Test unsupported requests and ensure the interface explains them rather than silently broadening the policy.

Exit only when suppression and preservation both pass. An unwanted action disappearing is insufficient if allowed keys are duplicated or the microphone is lost.

### Stage 6 Complete KernelSU integration

Add the ordinary module lifecycle, installation checks, safe defaults, supervisor, private state, and offline WebUI. Test the actual installed KernelSU implementation first and upstream KernelSU separately if they differ. Validate startup with no earphones and with earphones already connected.

Test boot, reconnect, live changes, update, disable, uninstall, and rescue. Test without a metamodule for the mountless backend when supported by the environment. Exit only with a KernelSU experimental release candidate and complete reference-phone evidence.

### Stage 7 Harden failure and privilege handling

Exercise worker crash and hang, lost uinput output, read failure, repeated restarts, root or policy denial, oversized configuration, full storage, concurrent control requests, and rapid reconnect. Review descriptor ownership, IPC authorization, file handling, and log privacy.

Exit only when every fault produces a bounded inactive state or a documented safe recovery, without hanging boot or suppressing essential input. Review every additional SELinux rule individually.

### Stage 8 Add Magisk support

Implement the smallest Magisk adapter and package compatibility changes. Test detection, service behavior, Action control, root command configuration, permission context, update, disable, and uninstall on a real supported Magisk environment.

Exit only when physical event and preservation tests pass on that environment. A ZIP accepted by Magisk, or a host installer simulation, is not sufficient evidence.

### Stage 9 Add APatch support

Implement and test APatch detection, lifecycle, paths, permissions, and supported configuration route. Inspect the exact version's metamodule requirements if any overlay is used. Reuse the same engine and policy schema.

Exit only with an APatch-specific physical result and truthful support entry. If no test device is available, retain APatch as a design target or experimental status with precise missing tests.

### Stage 10 Expand Android and hardware coverage

Test API 31 and the newest stable Android release being claimed, then add intermediate releases and ROM families according to available hardware. Add other USB-C headsets, a USB audio adapter with a wired headset, and, only with a validated backend, analog headset input.

Build additional ABIs only when there is a supported runtime and a test plan. Classify each combination separately. Test an unrelated keyboard or Bluetooth control path to verify isolation. Exit only when the public matrix matches the evidence.

### Stage 11 Prepare the release candidate

Freeze the feature set, close acceptance blockers, review documentation against actual behavior, generate the final ZIP and provenance from the reviewed commit, and test those exact bytes. Create a draft prerelease with checksums and limitations.

Run a bounded beta with explicit test cases and sanitized issue reports. Resolve critical input, audio, boot, or recovery failures before stable publication. Do not promote an experimental compatibility entry by changing wording alone.

### Stage 12 Publish and maintain

Follow Section 18 for publication when authorized by the maintainer. Monitor regression reports, keep the last known good release accessible, and correct support claims quickly. New device profiles, manager versions, and semantic adapters must go through focused pull requests and relevant physical validation.

## 16 Build and continuous integration requirements

### Reproducible build inputs

Pin the Android NDK, Rust toolchain, Cargo dependencies and checksums, TypeScript compiler, WebUI lockfile, and packaging tools. Target a native minimum platform compatible with API 31 and check that imported native APIs do not require a newer release. Build Rust binaries for Android targets with the NDK toolchain. [S26, S31]

Start with the measured ABI for the reference device rather than assuming it from a phone name. Include arm64-v8a and other supported ABIs as evidence and distribution goals warrant. Test executable loading, linker dependencies, and expected page-size compatibility on the Android releases being claimed. Do not assume a binary compiled for a desktop Linux ABI will run on Android.

For the native worker and any bundled shared libraries, configure and inspect ELF alignment suitable for 16 KiB page-size systems, avoid fixed page-size assumptions, and test on both 4 KiB and 16 KiB environments before claiming those environments. Android supports 16 KiB devices starting with Android 15. APK ZIP-alignment instructions apply only to an optional companion APK, not to the root module ZIP. The module's standalone native executables still need correct ELF loading behavior. [S18, S27]

Normalize archive ordering, timestamps, file modes, and text line endings. Record the source commit, toolchain versions, dependency lock state, and build recipe. Build twice in clean environments and compare digests. If a difference remains, identify its cause before claiming byte-for-byte reproducibility.

Generate an asset inventory, dependency notices, a software bill of materials where appropriate, and release checksums. Do not hide downloads or build-time patches outside the documented dependency process.

### Pull request checks

CI should check formatting and static analysis, portable shell syntax and lint, strict policy schema and migration tests, native unit tests, parser fuzz corpus regressions, host sanitizers, UI build and validation, and package structure. Test managers' detection logic against explicit fixtures, including conflicting and missing markers.

Build each distributed ABI and verify ELF architecture and unresolved dependencies. Inspect the packaged ZIP for the required root metadata, correct executable permissions, no enclosing repository directory, no private test data, no unknown binary payloads, and no privileged logic fetched at runtime.

Use a single final required CI status that fails if a mandatory dependency failed or was unexpectedly skipped. Keep device tests as separately recorded evidence; an emulator or unprivileged runner cannot prove physical USB behavior on PixelOS.

### Workflow security and release permissions

Give pull-request jobs read-only permissions and keep release write privileges in a separate job. Pin third-party actions to reviewed full commit SHAs. Do not run untrusted fork code in a privileged pull_request_target context or pass it release credentials. Validate manual workflow inputs as untrusted data. [S22]

Do not connect a personal rooted phone to arbitrary fork pull requests or an untrusted self-hosted runner. Hardware tests should run from reviewed code under maintainer control and avoid storing phone access credentials in the repository.

Use artifact attestations for release binaries when available and provide verification instructions. Attestations link an artifact to source and build context; they do not establish that the code is safe or that the hardware tests passed. [S23]

Separate CI artifact retention from durable public release assets. Never publish a random last-successful build or rebuild a release from a different branch after hardware acceptance.

## 17 Required validation matrix

### Minimum physical environment coverage

| Dimension | Required first evidence | Expansion rule |
| --- | --- | --- |
| KernelSU | User's exact implementation on the reference phone | Record upstream and each tested fork separately |
| Magisk | A real supported device or test installation | Full event, lifecycle, and preservation suite |
| APatch | A real supported device or test installation | Full event, lifecycle, and preservation suite |
| Android range | Reference release plus API 31 before claiming tested minimum | Newest stable release claimed and explicit intermediate coverage |
| Accessories | User's exact Xiaomi Type-C earphones | Add identified digital headsets and adapters independently |
| Native capability | uinput forwarding and permission success | Test all-control-only reduced mode separately if shipped |
| Isolation | Phone buttons and an unrelated input device | Add mixed-node and duplicate-device negative cases |

If hardware is unavailable, publish the corresponding row as untested or a design target. Do not install several root solutions simultaneously or alter the user's primary phone simply to populate the table. Separate test hardware or maintainer-arranged environments are preferable.

### Behavioral acceptance cases

| ID | Scenario | Required outcome |
| --- | --- | --- |
| T01 | Fresh installation | No interception before enrollment and explicit activation |
| T02 | Unknown accessory | Remains untouched; clear enrollment option |
| T03 | Block all | All observed earphone button actions suppressed in supported contexts |
| T04 | Each Custom group alone | Selected group blocked; independent unselected groups behave normally |
| T05 | Shared center semantics | Linked effects disclosed; unsupported distinctions not falsely offered |
| T06 | Short, long, repeat, double press | Policy consistently handles complete sequences and documented gestures |
| T07 | Screen on, locked, off | Supported behavior remains correct; wake limits disclosed |
| T08 | Incoming and ongoing test calls | Expected button policy; audio and microphone route preserved |
| T09 | Music and microphone recording | Playback quality and recording remain equivalent to baseline |
| T10 | Phone power and volume | Fully usable, including during earphone interception |
| T11 | Unrelated keyboard or control | Unchanged, including media keys on another device |
| T12 | Boot with and without earphones | No boot delay or wrong-device capture; correct eventual status |
| T13 | Reconnect with different event number | Identity resolved safely and policy reapplied |
| T14 | Unplug during a held key | No stuck virtual key or resource leak |
| T15 | Policy change during a press | Same decision throughout sequence; generation reported accurately |
| T16 | Worker crash and hang | Bounded release or supervised recovery; no persistent input loss |
| T17 | Missing uinput or permission denial | Safe inactive or explicitly selected tested reduced mode |
| T18 | Another program already has a grab | Precise conflict report; no attempt to defeat the other program |
| T19 | SYN_DROPPED and output failure | Correct reconciliation or bounded inactive state |
| T20 | Mixed node or misleading name | Rejected; essential switches and controls remain untouched |
| T21 | Corrupt or hostile policy and metadata | Rejected safely; no code execution or identity widening |
| T22 | Duplicate or composite devices | Only verified collections affected; ambiguity causes no activation |
| T23 | Update and migration | Valid preferences retained; no duplicate worker; rollback documented |
| T24 | Pause, manager disable, uninstall | Immediate pause works; manager lifecycle behavior documented and verified |
| T25 | ROM or kernel update | Changed profile evidence revalidated; unsupported environment reported |
| T26 | Idle and rapid reconnect stress | Performance budgets checked; no busy loop, excessive logs, or leak |
| T27 | Accessibility and offline UI | Settings and pause usable with assistive access and no network |
| T28 | Recovery drill | Maintainer can disable this module through a tested independent path |

Use at least 20 press and release cycles per supported independent group for release qualification, 20 reconnect cycles, and three boots in each tested boot configuration. These are project minimums, not proof of universal reliability. Add longer stress sessions when a regression or timing concern justifies them.

For every case record artifact digest, source commit, environment, profile, expected and actual result, pass or fail, and evidence location. Report not applicable with a concrete reason and distinguish not run. An allowed event must occur once with no added action; a blocked event must not reach its targeted action. Confirm preservation separately rather than inferring it from silence.

## 18 Release and GitHub publication workflow

1. Triage every open blocker. Input loss, broken audio or microphone, incorrect target selection, unsafe policy handling, and unreliable recovery block stable publication.
2. Update the compatibility matrix, README, recovery guide, changelog, dependency notices, and version metadata through a release pull request. Keep human versions and integer update ordering consistent across stable and prerelease channels.
3. Merge only after required checks and the applicable physical evidence are complete. Select the exact reviewed commit for the candidate.
4. Build the final assets from that clean commit with pinned tooling. Record digests before physical acceptance. If the build or packaged bytes change afterward, rerun the affected acceptance tests.
5. Create an annotated release tag on the accepted commit. Sign it when the maintainer has an established signing identity and verification process. Never invent a signature or move a published tag. Git documents the distinction between lightweight, annotated, and signed tags. [S24]
6. Prepare a draft GitHub release. Attach the installable module ZIP, checksums, dependency information, and available provenance. GitHub's automatic source ZIP is additional source material, not the installable asset.
7. Write release notes identifying supported Android and root environments, verified accessory profiles, independent configuration capabilities, linked-action limitations, installation and rollback paths, and known inactive windows or unsupported modes.
8. If immutable releases are enabled and available, attach every intended asset before publishing. Their assets and associated tag become locked after publication; GitHub recommends the draft-first workflow. [S25]
9. Publish only according to the maintainer's authorization. First release should be a prerelease until the beta acceptance criteria are met. Keep stable and experimental update channels separate if both exist.
10. After publication, verify the publicly downloadable asset digest and the source tag association. Validate each manager's update metadata against the published asset. Record the completed release in the repository.

Use the standard manager update mechanism where supported and tested. For Magisk's documented update descriptor, keep version, integer ordering, ZIP URL, and changelog tied to the same release. Verify actual KernelSU and APatch behavior rather than assuming identical handling. Do not add a separate background downloader or execute update payloads at boot. [S14]

Prefer a reviewed update pointer that resolves to a versioned release asset, not a mutable development ZIP. HTTPS and checksums help distribution integrity, but a checksum hosted beside the same asset does not independently authenticate its publisher. Provide provenance or signature verification where available.

For a faulty release, publish a clear advisory and a corrected version or rollback instructions. Retain the known-good artifact and its test record. Do not silently replace the bytes behind an existing version, move its tag, or claim a fixed package has the old tested digest.

## 19 Required project documentation and maintenance

The README must explain the feature in plain language, priority root platforms, tested Android range, prerequisites, configuration, and what stays functional. State that selection is accessory-specific and that some apparent actions share one raw signal. Put a working installation and recovery path where users can find it quickly.

Maintain dedicated documents for architecture, policy schema and migrations, profile enrollment, root-manager adapters, hardware testing, compatibility, build reproducibility, release process, privacy, and troubleshooting. Keep examples in the implemented repository aligned with the real tools; this specification intentionally provides no executable examples.

Create architecture decision records for native interception versus overlays, proxy mapping, target matching, shared-action semantics, supervisor recovery, policy persistence, and release signing or attestation. Each decision should record the problem, evidence, chosen behavior, alternatives that matter, and conditions for revisiting it.

Provide issue templates for a new accessory profile, unsupported environment, blocked control leaking through, allowed control broken, audio or microphone regression, boot or recovery problem, and security report. Keep security disclosure private through the documented channel when possible.

Review new root-manager releases, Android changes, and dependency updates before widening support. Profiles should include the evidence and version constraints needed to invalidate them safely. Remove or downgrade stale support claims when they cannot be reproduced.

## 20 Definition of done and agent handoff

The implementing agent may call a release stable only when all of the following are true:

- Source and packaging are complete, licensed, reviewable, and tied to an exact release commit.
- KernelSU is the first validated platform; Magisk and APatch have their own truthful evidence and support status.
- The reference Xiaomi Type-C earphones on the user's actual PixelOS and KernelSU setup have a measured profile and passed the physical tests before being listed as verified.
- Block all and independently supported Custom groups work, with shared or unavailable actions clearly explained.
- Phone controls, audio, microphone, connection handling, and unrelated inputs pass preservation tests.
- Defaults, unknown-device handling, SELinux behavior, crash and hang handling, and recovery are safe and documented.
- Configuration persists through tested updates, reports the effective policy, and cannot execute untrusted text.
- CI, reproducible build evidence, hardware records, compatibility matrix, release notes, and verification assets agree with the shipped bytes.
- Public claims state the tested Android and device range without a universal guarantee.

At handoff, report what was implemented, what was physically tested, what remains experimental, the exact commit and artifact digests, links to relevant pull requests and releases, and any maintainer action still needed. If access to hardware or publication credentials is missing, complete the reproducible build and reviewable draft, clearly identify the missing validation, and do not substitute an invented result.

## 21 Official sources and verification map

Sources S01 through S30 were consulted on 3 November 2026; S31 was consulted on 4 November 2026. They are primary project documentation or AOSP source. Pin relevant revisions when implementing. The identity and language stack were approved on 4 November 2026. Design choices, test budgets, defaults, and release gates are project requirements rather than upstream guarantees.

- [S01] Android Open Source Project. Key layout files. Layout lookup, device-specific mappings, HID usages, and the warning against changing Generic.kl. [Official source](https://source.android.com/docs/core/interaction/input/key-layout-files)
- [S02] Android Open Source Project. Input overview. Distinguishes physical scan codes, HID usages, Linux codes, Android keycodes, and mapping layers. [Official source](https://source.android.com/docs/core/interaction/input)
- [S03] Android Open Source Project. USB headset Accessory specification. Control functions and HID mapping references. [Official source](https://source.android.com/docs/core/interaction/accessories/headset/usb-headset-spec)
- [S04] Android Open Source Project. USB headset Device specification. Linux and Android mappings and context dependent media and call behavior. [Official source](https://source.android.com/docs/core/interaction/accessories/headset/usb-device)
- [S05] Android Open Source Project. getevent tool. Input-device capability inspection and kernel-event diagnostics. [Official source](https://source.android.com/docs/core/interaction/input/getevent)
- [S06] Linux kernel documentation. uinput module. Virtual device creation and the recommendation to consider libevdev. [Official source](https://docs.kernel.org/input/uinput.html)
- [S07] Linux kernel documentation. Input event codes. Event types, press and release values, synchronization, and overrun recovery. [Official source](https://docs.kernel.org/input/event-codes.html)
- [S08] libevdev project documentation. Initialization and setup. Device grab and ungrab through EVIOCGRAB. [Official source](https://www.freedesktop.org/software/libevdev/doc/latest/group__init.html)
- [S09] AOSP frameworks native source. EventHub.cpp on the main branch. Device mappings, key character maps, and layout processing. [Official source](https://android.googlesource.com/platform/frameworks/native/+/refs/heads/main/services/inputflinger/reader/EventHub.cpp)
- [S10] AOSP frameworks native source. KeyboardInputMapper.cpp on the main branch. Processing of mapped and unknown keys. [Official source](https://android.googlesource.com/platform/frameworks/native/+/refs/heads/main/services/inputflinger/reader/mapper/KeyboardInputMapper.cpp)
- [S11] KernelSU. Module guide. Metadata, install environment, supported hooks, manager compatibility variables, and mountless versus mounted features. [Official source](https://kernelsu.org/guide/module.html)
- [S12] KernelSU. Metamodule guide. Mounting-provider architecture and its dependencies. [Official source](https://kernelsu.org/guide/metamodule.html)
- [S13] KernelSU. Module WebUI. Offline module resources, entry point, bridge, and persistence considerations. [Official source](https://kernelsu.org/guide/module-webui.html)
- [S14] Magisk. Developer Guides. Module metadata, scripts, installation conventions, systemless overlays, and update metadata. [Official source](https://topjohnwu.github.io/Magisk/guides.html)
- [S15] KernelSU. Rescue from bootloop. Official rescue options and integration or timing qualifications. [Official source](https://kernelsu.org/guide/rescue-from-bootloop.html)
- [S16] APatch. AndroidPatch Module Development Guide. APM structure, environment and lifecycle differences. [Official source](https://apatch.dev/apm-guide.html)
- [S17] APatch. Metamodules. Current mounting-provider documentation. [Official source](https://apatch.dev/meta-module.html)
- [S18] Android Developers. Android ABIs. Native binary architectures and ABI considerations. [Official source](https://developer.android.com/ndk/guides/abis)
- [S19] Magisk. Frequently Asked Questions. Official recovery and safe-mode guidance. [Official source](https://topjohnwu.github.io/Magisk/faq.html)
- [S20] APatch. Rescue from bootloop. Official module recovery guidance. [Official source](https://apatch.dev/rescue-bootloop.html)
- [S21] GitHub Docs. About protected branches. Required checks, reviews, and branch protection capabilities. [Official source](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches)
- [S22] GitHub Docs. Secure use reference. Workflow permissions, third-party actions, and untrusted workflow execution. [Official source](https://docs.github.com/en/actions/reference/security/secure-use)
- [S23] GitHub Docs. Artifact attestations. Build provenance, verification, and limits of attestations. [Official source](https://docs.github.com/en/actions/concepts/security/artifact-attestations)
- [S24] Git project. Pro Git Tagging. Annotated and signed tags. [Official source](https://git-scm.com/book/en/v2/Git-Basics-Tagging)
- [S25] GitHub Docs. Immutable releases. Protected assets and tags and draft-first publication. [Official source](https://docs.github.com/en/code-security/concepts/supply-chain-security/immutable-releases)
- [S26] Android Developers. Using newer APIs. Native API availability and load-time compatibility. [Official source](https://developer.android.com/ndk/guides/using-newer-apis)
- [S27] Android Developers. Support 16 KB page sizes. Native alignment, page-size assumptions, and validation environments. [Official source](https://developer.android.com/guide/practices/page-sizes)
- [S28] Magisk source. module_installer.sh on the master branch. Upstream installer wrapper and delegation to supported installer functions. [Official source](https://github.com/topjohnwu/Magisk/blob/master/scripts/module_installer.sh)
- [S29] Android Open Source Project. Input device configuration files. Device-specific configuration, input classification, and lookup. [Official source](https://source.android.com/docs/core/interaction/input/input-device-configuration-files)
- [S30] Android Open Source Project. Security Enhanced Linux in Android. Mandatory access control applies even to processes with root privileges. [Official source](https://source.android.com/docs/security/features/selinux)
- [S31] Rust project. Android target support. Native Android targets, NDK integration, and building Rust binaries. [Official source](https://doc.rust-lang.org/rustc/platform-support/android.html)
