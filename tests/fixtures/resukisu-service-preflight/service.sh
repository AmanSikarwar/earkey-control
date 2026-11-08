#!/system/bin/sh
set -eu

case "$0" in
    */service.sh) cd "${0%/*}" ;;
    *) exit 1 ;;
esac

case "${1:-}" in
    '') stage=service; attempt=.preflight ;;
    --boot-completed) stage=boot_completed; attempt=.boot-preflight ;;
    *) exit 1 ;;
esac
[ "$#" -le 1 ] || exit 1

for flag in disable remove; do
    if [ -e "$flag" ] || [ -L "$flag" ]; then
        exit 0
    fi
done

umask 077
# Reserve the only attempt before checking or starting the payload.
mkdir -m 0700 "$attempt" 2>/dev/null || exit 0
(
    uptime=
    if IFS=' ' read -r uptime _ 2>/dev/null < /proc/uptime && [ "${#uptime}" -le 20 ]; then
        case "$uptime" in
            *.*)
                whole=${uptime%%.*}; fraction=${uptime#*.}
                case "$whole" in
                    ''|*[!0123456789]*) uptime=unavailable ;;
                esac
                case "$fraction" in
                    ''|*[!0123456789]*) uptime=unavailable ;;
                esac
                ;;
            *) uptime=unavailable ;;
        esac
    else
        uptime=unavailable
    fi
    printf 'hook_uptime_seconds=%s\n' "$uptime"
    context=
    if IFS= read -r context 2>/dev/null < "/proc/$$/attr/current" || [ -n "$context" ]; then
        case "$context" in
            ''|*[!a-zA-Z0-9_:.,-]*) printf 'selinux_context=invalid\n' ;;
            *)
                if [ "${#context}" -le 128 ]; then
                    printf 'selinux_context=%s\n' "$context"
                else
                    printf 'selinux_context=invalid\n'
                fi
                ;;
        esac
    else
        printf 'selinux_context=unavailable\n'
    fi
    for ns in mnt uts; do
        ns_target=$(timeout -s KILL 5 /system/bin/readlink "/proc/$$/ns/$ns" 2>/dev/null) || ns_target=
        case "$ns_target" in
            "$ns":\[*\])
                ns_id=${ns_target#*\[}; ns_id=${ns_id%\]}
                case "$ns_id" in
                    ''|*[!0123456789]*) ns_id=unavailable ;;
                    *) [ "${#ns_id}" -le 20 ] || ns_id=unavailable ;;
                esac
                ;;
            *) ns_id=unavailable ;;
        esac
        printf '%s_namespace=%s\n' "$ns" "$ns_id"
    done
    # Inspect only this hook shell; unrelated descriptor targets are never printed.
    scan=complete; scanned=0
    driver_yes=0; driver_no=0; driver_unknown=0
    su_yes=0; su_no=0; su_unknown=0
    for fd in "/proc/$$/fd/"*; do
        scanned=$((scanned + 1))
        if [ "$scanned" -gt 16 ]; then scan=incomplete; break; fi
        if [ ! -L "$fd" ]; then scan=incomplete; continue; fi
        target=$(timeout -s KILL 1 /system/bin/readlink "$fd" 2>/dev/null) || { scan=incomplete; continue; }
        case "$target" in
            'anon_inode:[ksu_driver]') kind=driver ;;
            'anon_inode:[ksu_driver_su]') kind=su ;;
            *) continue ;;
        esac
        number=${fd##*/}; flags=; flag_count=0
        if [ -r "/proc/$$/fdinfo/$number" ]; then
            while IFS= read -r line; do
                case "$line" in
                    flags:*)
                        set -f
                        # flags fields require word splitting; pathname expansion is disabled.
                        # shellcheck disable=SC2086
                        set -- $line
                        set +f
                        flag_count=$((flag_count + 1))
                        [ "$#" = 2 ] && flags=$2 || flags=
                        ;;
                esac
            done < "/proc/$$/fdinfo/$number"
        fi
        close=unknown
        case "$flags" in
            ''|*[!01234567]*) ;;
            *)
                if [ "$flag_count" = 1 ] && [ "${#flags}" -le 10 ]; then
                    if [ $((0$flags & 02000000)) -ne 0 ]; then close=yes; else close=no; fi
                fi
                ;;
        esac
        case "$kind:$close" in
            driver:yes) driver_yes=$((driver_yes + 1)) ;;
            driver:no) driver_no=$((driver_no + 1)) ;;
            driver:unknown) driver_unknown=$((driver_unknown + 1)) ;;
            su:yes) su_yes=$((su_yes + 1)) ;;
            su:no) su_no=$((su_no + 1)) ;;
            su:unknown) su_unknown=$((su_unknown + 1)) ;;
        esac
    done
    printf 'fd_scan=%s\ndriver_cloexec_yes=%s\ndriver_cloexec_no=%s\ndriver_cloexec_unknown=%s\n' "$scan" "$driver_yes" "$driver_no" "$driver_unknown"
    printf 'driver_su_cloexec_yes=%s\ndriver_su_cloexec_no=%s\ndriver_su_cloexec_unknown=%s\n' "$su_yes" "$su_no" "$su_unknown"
) > "$attempt/context.tmp" 2>&1
mv "$attempt/context.tmp" "$attempt/context"
(
    printf 'hook=entered\n'
    status=1

    query_manager() {
        # Query only the reviewed manager; this does not relax a failed guard.
        if [ "${KSU_UAPI_VER:-}" = 6 ] && [ "$(id -u)" = 0 ] &&
           [ -f /data/adb/ksud ] && [ ! -L /data/adb/ksud ] &&
           [ -x /data/adb/ksud ] &&
           [ "$(stat -c '%u:%g:%a' /data/adb/ksud)" = 0:0:755 ] &&
           manager_hash=$(timeout -s KILL 5 /system/bin/sha256sum /data/adb/ksud) &&
           [ "${manager_hash%% *}" = fa0a460a91dc65b4408f9eb63a883342e0b017e84da5e444dee4903ee31d8f50 ]; then
            if manager_result=$(timeout -s KILL 5 /data/adb/ksud debug version 2>/dev/null); then
                case "$manager_result" in
                    'Kernel Version: '*)
                        manager_version=${manager_result#'Kernel Version: '}
                        case "$manager_version" in
                            ''|*[!0123456789]*) printf 'fresh_query=invalid\n' ;;
                            *)
                                if [ "${#manager_version}" -le 10 ]; then
                                    printf 'fresh_query=numeric\nfresh_query_value=%s\n' "$manager_version"
                                else
                                    printf 'fresh_query=invalid\n'
                                fi
                                ;;
                        esac
                        ;;
                    *) printf 'fresh_query=invalid\n' ;;
                esac
            else
                printf 'fresh_query=failed\n'
            fi
        else
            printf 'fresh_query=unavailable\n'
        fi
    }

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
                query_manager
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
        if [ "$stage" = boot_completed ]; then
            printf 'kernel_version_state=numeric\nkernel_version_value=35240\n'
            query_manager
            status=0
        elif [ -f ./probe.sha256 ] && [ ! -L ./probe.sha256 ] &&
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
    if [ "$stage" = boot_completed ]; then
        printf 'guard_exit=%s\n' "$status"
    else
        printf 'service_probe_exit=%s\n' "$status"
    fi
) > "$attempt/result.tmp" 2>&1
mv "$attempt/result.tmp" "$attempt/result"
