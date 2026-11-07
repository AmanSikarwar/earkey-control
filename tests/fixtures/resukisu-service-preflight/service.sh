#!/system/bin/sh
set -eu

case "$0" in
    */service.sh) cd "${0%/*}" ;;
    *) exit 1 ;;
esac

for flag in disable remove; do
    if [ -e "$flag" ] || [ -L "$flag" ]; then
        exit 0
    fi
done

umask 077
# Reserve the only attempt before checking or starting the payload.
mkdir -m 0700 .preflight 2>/dev/null || exit 0
(
    printf 'hook=entered\n'
    status=1

    guard() {
        if [ "$2" != "$3" ]; then
            printf 'guard_failure=%s\n' "$1"
            if [ "$1" = kernel_version ]; then
                if [ "${KSU_KERNEL_VER_CODE+x}" != x ]; then
                    printf 'kernel_version_state=unset\n'
                elif [ -z "$2" ]; then
                    printf 'kernel_version_state=empty\n'
                else
                    case "$2" in
                        *[!0123456789]*) printf 'kernel_version_state=invalid\n' ;;
                        *)
                            if [ "${#2}" -le 10 ]; then
                                printf 'kernel_version_state=numeric\nkernel_version_value=%s\n' "$2"
                            else
                                printf 'kernel_version_state=invalid\n'
                            fi
                            ;;
                    esac
                fi
            fi
            return 1
        fi
    }

    if guard ksu "${KSU:-}" true &&
       guard bakasu "${BAKASU:-}" true &&
       guard module "${KSU_MODULE:-}" earkey_control_preflight &&
       guard userspace_version "${KSU_VER_CODE:-}" 35257 &&
       guard kernel_version "${KSU_KERNEL_VER_CODE:-}" 35240 &&
       guard uapi_version "${KSU_UAPI_VER:-}" 6 &&
       guard uid "$(id -u)" 0; then
        if [ -f ./probe.sha256 ] && [ ! -L ./probe.sha256 ] &&
           [ "$(stat -c '%s' ./probe.sha256)" = 65 ] &&
           [ -f ./earkey-probe ] && [ ! -L ./earkey-probe ] && [ -x ./earkey-probe ] &&
           IFS= read -r expected < ./probe.sha256 &&
           actual=$(timeout -s KILL 5 /system/bin/sha256sum ./earkey-probe) &&
           [ "${actual%% *}" = "$expected" ]; then
            printf 'launcher=resukisu_service\n'
            printf 'artifact_sha256=%s\n' "$expected"
            status=0
            timeout -s KILL 5 ./earkey-probe self-check || status=$?
        else
            printf 'preflight=digest_mismatch\n'
        fi
    fi
    printf 'service_probe_exit=%s\n' "$status"
) > .preflight/result.tmp 2>&1
mv .preflight/result.tmp .preflight/result
