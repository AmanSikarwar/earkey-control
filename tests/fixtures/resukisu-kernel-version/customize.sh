#!/system/bin/sh

# This script is sourced by the measured fork's native boot-mode installer.
# The kernel-version value is deliberately not checked here: the diagnostic
# exists to explain why that value differs between hook stages.
if [ "${BOOTMODE:-}" != true ] ||
   [ "${KSU:-}" != true ] ||
   [ "${BAKASU:-}" != true ] ||
   [ "${KSU_VER_CODE:-}" != 35257 ] ||
   [ "${KSU_UAPI_VER:-}" != 6 ] ||
   [ "${ARCH:-}" != arm64 ] ||
   [ "${API:-}" != 37 ] ||
   [ "${MODPATH:-}" != /data/adb/modules_update/earkey_control_kver ]; then
    abort "This diagnostic requires the reference ReSukiSU boot-mode installer."
fi
