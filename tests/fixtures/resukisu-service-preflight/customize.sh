#!/system/bin/sh

# This script is sourced by the measured fork's native boot-mode installer.
if [ "${BOOTMODE:-}" != true ] ||
   [ "${KSU:-}" != true ] ||
   [ "${BAKASU:-}" != true ] ||
   [ "${KSU_VER_CODE:-}" != 35257 ] ||
   [ "${KSU_KERNEL_VER_CODE:-}" != 35240 ] ||
   [ "${KSU_UAPI_VER:-}" != 6 ] ||
   [ "${ARCH:-}" != arm64 ] ||
   [ "${API:-}" != 37 ] ||
   [ "${MODPATH:-}" != /data/adb/modules_update/earkey_control_preflight ]; then
    abort "This diagnostic requires the reference ReSukiSU boot-mode installer."
fi

set_perm "$MODPATH/earkey-probe" 0 0 0500 || abort "Unable to set probe permissions."
