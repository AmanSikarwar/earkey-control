#!/system/bin/sh
# Sourced by the KernelSU boot-mode installer. Fresh installs stay inactive.

[ "${BOOTMODE:-}" = true ] || abort "! Install from the root manager while Android is running."
[ "${KSU:-}" = true ] || abort "! EarKey Control currently supports KernelSU only."
[ "${ARCH:-}" = arm64 ] || abort "! EarKey Control needs an arm64 device."
case "${API:-}" in
    ''|*[!0-9]*) abort "! Unknown Android API level." ;;
esac
[ "$API" -ge 31 ] || abort "! EarKey Control needs Android 12 (API 31) or later."

set_perm_recursive "$MODPATH" 0 0 0755 0644 || abort "! Unable to set module permissions."
set_perm "$MODPATH/bin/earkey-worker" 0 0 0500 || abort "! Unable to set worker permissions."
set_perm "$MODPATH/service.sh" 0 0 0755 || abort "! Unable to set service permissions."
set_perm "$MODPATH/uninstall.sh" 0 0 0755 || abort "! Unable to set uninstall permissions."

storage=/data/adb/earkey_control
[ -L "$storage" ] && abort "! $storage must not be a symlink."
if [ ! -d "$storage" ]; then
    mkdir "$storage" || abort "! Unable to create $storage."
fi
if ! chown 0:0 "$storage" || ! chmod 0700 "$storage"; then
    abort "! Unable to secure $storage."
fi

ui_print "- EarKey Control is installed and inactive."
ui_print "- After reboot, with the earphones connected, run as root:"
ui_print "-   $MODPATH/bin/earkey-worker enroll"
ui_print "-   $MODPATH/bin/earkey-worker apply block-all enable"
